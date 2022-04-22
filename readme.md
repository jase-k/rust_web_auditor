# Purpose: 

The purpose of this tool is to allow web developers to get a high level overview of website performance, and track 404 links. More functionality to come. . . 

# Setup: 

Using Gecko Driver:
1. If Firefox is not installed. Install: https://www.mozilla.org/en-US/firefox/new/
*On linux you can install firefox by ```sudo apt install firefox```*
2. Download and install geckodriver: https://github.com/mozilla/geckodriver (downloads under releases)
*optionally install by ```cargo install geckdriver```*
3. Open the executable and confirm webdriver is running on port 4444
![geckodriver_example](./docs/images/geckodriver_example.PNG)

# Linux Setup: 
Install Rust: https://www.rust-lang.org/tools/install
```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Install Openssl: 
```
sudo apt update
sudo apt install openssl
sudo apt install libssl-dev
```