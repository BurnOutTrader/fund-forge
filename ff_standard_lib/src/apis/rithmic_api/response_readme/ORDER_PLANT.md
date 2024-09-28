
## Accounts List
```rust
fn example() {
    let accounts = RequestAccountList {
        template_id: 302,
        user_msg: vec![],
        fcm_id: None,
        ib_id: None,
        user_type: Some(UserType::Trader.into())
    };
}
``` 
***Accounts Response***
Account List Response (Template ID: 303) from Server: ResponseAccountList { template_id: 303, user_msg: [], rq_handler_rp_code: ["0"], rp_code: [], fcm_id: Some("TopstepTrader"), ib_id: Some("TopstepTrader"), account_id: Some("S1Sep246906077"), account_name: Some("S1Sep246906077"), account_currency: Some("USD"), account_auto_liquidate: Some("enabled"), auto_liq_threshold_current_value: None }

## Account RMS Info
```rust
fn example() {
    let rms_req = RequestAccountRmsInfo {
        template_id: 304,
        user_msg: vec![],
        fcm_id: None,
        ib_id: None,
        user_type: Some(UserType::Trader.into()),
    };
}
```
***Account RMS Response***
Response Account Rms Info (Template ID: 305) from Server: ResponseAccountRmsInfo { template_id: 305, user_msg: [], rq_handler_rp_code: ["0"], 
rp_code: [], presence_bits: Some(1023), fcm_id: Some("TopstepTrader"), ib_id: Some("TopstepTrader"), account_id: Some("S1Sep246906077"), 
currency: Some("USD"), status: Some("active"), algorithm: Some("Max Loss"), auto_liquidate_criteria: Some("Multiple Simultaneous Criteria"), 
auto_liquidate: Some(Enabled), disable_on_auto_liquidate: None, auto_liquidate_threshold: Some(1000.0), auto_liquidate_max_min_account_balance: 
Some(50000.0), loss_limit: Some(1000.0), min_account_balance: Some(0.0), min_margin_balance: Some(0.0), default_commission: None, 
buy_limit: Some(5), max_order_quantity: Some(10), sell_limit: Some(5), check_min_account_balance: Some(false) }

## Login Info 
This is useful to get fcm_id and ib_id for other request types and to create login template
```rust
RequestLoginInfo {
    template_id: 300 ,
    user_msg: vec![],
};
```
***Account RMS Response***
Response Login Info (Template ID: 303) from Server: ResponseLoginInfo { template_id: 301, user_msg: [], rp_code: ["0"], fcm_id: Some("xxx"), ib_id: Some("xxxx"), 
first_name: Some("xx"), last_name: Some("xxx"), user_type: Some(Trader) }

    
    




