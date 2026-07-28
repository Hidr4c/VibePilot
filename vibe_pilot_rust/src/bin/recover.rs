fn main() {
    println!("Checking keyring error details...");
    let entry = keyring::Entry::new("VibePilot", "VibePilot_Master_Key");
    match entry {
        Ok(e) => {
            println!("Entry created successfully.");
            match e.get_password() {
                Ok(pwd) => println!("Password found: {} chars", pwd.len()),
                Err(err) => println!("get_password error: {:?}", err),
            }
            // Test setting a password to see if it fails
            match e.set_password("test_pwd") {
                Ok(_) => {
                    println!("set_password succeeded!");
                    let _ = e.delete_credential();
                }
                Err(err) => println!("set_password error: {:?}", err),
            }
        }
        Err(err) => println!("Entry creation error: {:?}", err),
    }
}
