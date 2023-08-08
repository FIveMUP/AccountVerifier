use std::{fs, sync::Arc, io::Write};
use fantoccini::{ClientBuilder, Locator};
use tokio::{sync::{Mutex, Semaphore}, task};
use anyhow::Result;

#[derive(Clone)]
struct Account {
    pub mail: String,
    pub password: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let accounts = load_accounts("accounts.txt")?;
    let verified_accounts = Arc::new(Mutex::new(load_verified_accounts("output_accounts.txt")?));
    println!("Verified accounts loaded: {}", verified_accounts.lock().await.len());
    println!("ToCheck accounts loaded: {}", accounts.len());

    let sem = Arc::new(Semaphore::new(7)); // Máximo de 10 tareas concurrentes

    let mut handles = vec![];

    for my_account in accounts {
        let sem_clone = Arc::clone(&sem);
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        let verified_accounts_clone = Arc::clone(&verified_accounts);

        let handle = task::spawn(async move {
            let _permit = sem_clone.acquire().await;
            if verified_accounts_clone.lock().await.len() > 1 {
                let mut file = fs::File::create("output_accounts.txt")?;
                println!("Total verified accounts: {}", verified_accounts_clone.lock().await.len());
                for account in verified_accounts_clone.lock().await.iter() {
                    file.write_all(format!("{}:{}\n", account.mail, account.password).as_bytes())?;
                }
            }
            
            process_account(my_account, &verified_accounts_clone).await
        });
        handles.push(handle);
    }

    // Espera a que todas las tareas se completen
    for handle in handles {
        println!("Task completed");
        let _ = handle.await?;
    }

    let verified_accounts_clone = Arc::clone(&verified_accounts);

    if verified_accounts_clone.lock().await.len() > 1 {
        let mut file = fs::File::create("output_accounts.txt")?;
        println!("Total verified accounts: {}", verified_accounts_clone.lock().await.len());
        for account in verified_accounts_clone.lock().await.iter() {
            file.write_all(format!("{}:{}\n", account.mail, account.password).as_bytes())?;
        }
    }

    Ok(())
}

fn load_accounts(filename: &str) -> Result<Vec<Account>, std::io::Error> {
    let bindings = fs::read_to_string(filename)?;
    Ok(bindings.split('\n').filter_map(|line| {
        let parts: Vec<_> = line.split(':').collect();
        if parts.len() >= 2 {
            Some(Account {
                mail: parts[0].to_string(),
                password: parts[1].to_string(),
            })
        } else {
            None
        }
    }).collect())
}

fn load_verified_accounts(filename: &str) -> Result<Vec<Account>, std::io::Error> {
    let contents = fs::read_to_string(filename)?;
    Ok(contents.split('\n').filter_map(|line| {
        let parts: Vec<_> = line.split(':').collect();
        if parts.len() >= 2 {
            Some(Account {
                mail: parts[0].to_string(),
                password: parts[1].to_string(),
            })
        } else {
            None
        }
    }).collect())
}

async fn process_account(account: Account, verified_accounts: &Arc<Mutex<Vec<Account>>>) -> Result<()> {
    if verified_accounts.lock().await.iter().any(|a| a.mail == account.mail) {
        println!("Account {} is already verified", account.mail);
        return Ok(());
    }

    let msec = rand::random::<u64>() % 700 + 100;

    tokio::time::sleep(std::time::Duration::from_millis(msec)).await;

    let html = check_account_verification(&account).await?;

    if html.contains("Puede que tu cuenta ya haya sido verificada") {
        println!("Account {} is already verified", account.mail);
        verified_accounts.lock().await.push(account);
    } else if html.contains("Tu correo electrónico se ha verificado") {
        println!("Account {} is verified", account.mail);
        verified_accounts.lock().await.push(account);
    } else {
        println!("Error with account {}", account.mail);
    }

    Ok(())
}

async fn check_account_verification(account: &Account) -> Result<String> {
    let c = ClientBuilder::rustls().connect("http://localhost:9515").await?;
    let x: u32 = rand::random::<u32>() % 1920;
    let y = rand::random::<u32>() % 1080;
    c.set_window_position(x, y).await?;
    c.set_window_size(400, 400).await?;
    c.goto("https://mail.projectnoxius.com/webmail/").await?;
    // select random position for window
    // rand nubmer between 0, and 1920
    c.find(Locator::Css("input[id='rcmloginuser']")).await?.send_keys(&account.mail).await?;
    c.find(Locator::Css("input[id='rcmloginpwd']")).await?.send_keys(&account.password).await?;
    c.find(Locator::Css("button[id='rcmloginsubmit']")).await?.click().await?;
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let verify_mail = c.find(Locator::LinkText("Verify your Rockstar Games Social Club email address")).await?;
    let href = verify_mail.attr("href").await?.unwrap().to_string();
    c.goto(&href).await?;
    tokio::time::sleep(std::time::Duration::from_millis(3500)).await;

    let verify_mail_final = c.find(Locator::Css("a[href^='https://socialclub.rockstargames.com/profile/verify']")).await?;
    let href = verify_mail_final.attr("href").await?.unwrap().to_string();
    c.goto(&href).await?;
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    Ok(c.source().await?)
}
