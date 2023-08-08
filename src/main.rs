use std::{fs, f32::consts::E, sync::Arc};
use fantoccini::{ClientBuilder, Locator, wd::TimeoutConfiguration};
use tokio::sync::Mutex;

#[tokio::main]
async fn main () -> Result<(), Box<dyn std::error::Error>> {
    
    let accounts = std::fs::read_to_string("accounts.txt");

    struct Account {
        pub mail: String,
        pub password: String
    }

    if accounts.is_err() {
        println!("Error reading accounts.txt");
        fs::write("accounts.txt", "").expect("Unable to write file");
        let accounts = std::fs::read_to_string("accounts.txt").expect("Unable to read file");
    }

    let bindings = accounts.unwrap();
    let accounts = bindings.split("\n").collect::<Vec<&str>>();
    let accounts: Vec<Account> = accounts.iter().map(|x| {
        let account = x.split(":").collect::<Vec<&str>>();
        Account {
            mail: account[0].to_string(),
            password: account[1].to_string()
        }
    }).collect::<Vec<Account>>();

    // Mutex to prevent multiple threads from accessing the same account
    let already_verified_accounts = std::fs::read_to_string("output_accounts.txt");
    let verified_accounts = Arc::new(Mutex::new(Vec::<Account>::new()));

    for account in already_verified_accounts.unwrap().split("\n") {
        let account = account.split(":").collect::<Vec<&str>>();
        if account.len() < 2 {
            continue;
        }
        verified_accounts.lock().await.push(Account {
            mail: account[0].to_string(),
            password: account[1].to_string()
        });
    }
    
    for my_account in accounts  {
        println!("Starting for {}", my_account.mail);
        let mail = my_account.mail.clone();
        let password = my_account.password.clone();
        let verified_accounts_length = verified_accounts.lock().await.len();

        if verified_accounts_length > 0 {
            println!("Total of: {} accounts verified, flashing file...", verified_accounts_length);
            let mut output_accounts = String::new();
            for account in verified_accounts.lock().await.iter() {
                output_accounts.push_str(&format!("{}:{}\n", account.mail, account.password));
            }
            fs::write("output_accounts.txt", output_accounts).expect("Unable to write file");
        }

        let c = ClientBuilder::rustls().connect("http://localhost:9515").await.expect("failed to connect to WebDriver");

        c.goto("https://mail.projectnoxius.com/webmail/").await?;

        c.execute("window.focus();", vec![]).await?;

        c.find(Locator::Css("input[id='rcmloginuser']")).await?.send_keys(&mail).await?;
        c.find(Locator::Css("input[id='rcmloginpwd']")).await?.send_keys(&password).await?;
        c.find(Locator::Css("button[id='rcmloginsubmit']")).await?.click().await?;
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        // search span with content "gaInbvox"
        let verify_mail = c.find(Locator::LinkText(("Verify your Rockstar Games Social Club email address"))).await?;
        let href = verify_mail.attr("href").await?.unwrap().to_string();
        c.goto(&href).await?;



        println!("Found verify mail {:?}", href);
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let verify_mail_final = c.find(Locator::Css("a[href^='https://socialclub.rockstargames.com/profile/verify']")).await?;
        let href = verify_mail_final.attr("href").await?.unwrap().to_string();
        c.goto(&href).await?;
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let html = c.source().await?;

        if html.contains("Puede que tu cuenta ya haya sido verificada") {
            println!("Account {} is already verified", mail);
            continue;
        } else if html.contains("Tu correo electrónico se ha verificado") {
            verified_accounts.lock().await.push(my_account);
            continue;
        } else {
            println!("Error with account {}", mail);
            tokio::time::sleep(std::time::Duration::from_secs(44345343)).await;    
            continue;
        }

    }





    Ok(())
}