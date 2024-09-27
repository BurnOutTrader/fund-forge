
# Subscribing and Unsubscribing Data
```rust
fn example() {
    let req = RequestMarketDataUpdate {
        template_id: 100,
        user_msg: vec![],
        symbol: Some("NQ".to_string()),
        exchange: Some(Exchange::CME.to_string()), // we can just convert fund forge exchange object to string
        request: Some(Request::Subscribe.into()), // Request is subscribe or unsubscribe
        update_bits: Some(4096), //enum UpdateBits
    };
}
```
## Options and Outputs
***UpdateBits*** 
LastTrade = 1,
streams last trades
```
LastTrade { template_id: 150, symbol: Some("NQ"), exchange: Some("CME"), presence_bits: Some(23),
clear_bits: None, is_snapshot: None, trade_price: Some(20293.75), trade_size: Some(1), aggressor: Some(Buy), exchange_order_id: Some("6856154850136"),
aggressor_exchange_order_id: Some("6856154852688"), net_change: Some(-52.5), percent_change: Some(-0.26), volume: None, vwap: Some(20316.5), trade_time: None,
ssboe: Some(1727421634), usecs: Some(36187), source_ssboe: Some(1727421634), source_usecs: Some(35749), source_nsecs: Some(35749949), jop_ssboe: Some(1727421634),
jop_nsecs: Some(36053625) }
```

Bbo = 2, //best bid offer
```
BestBidOffer { template_id: 151, symbol: Some("NQ"), exchange: Some("CME"), presence_bits: Some(5), clear_bits: None, 
is_snapshot: None, bid_price: Some(20285.0), bid_size: Some(2), bid_orders: Some(2), bid_implicit_size: Some(0), 
bid_time: None, ask_price: None, ask_size: None, ask_orders: None, ask_implicit_size: None, ask_time: None, 
lean_price: Some(20285.5), ssboe: Some(1727421767), usecs: Some(121760) }
```

OrderBook = 4, //order book, need level 2 subscription for rithmic

Open = 8, //days open?

OpeningIndicator = 16, //not useful

HighLow = 32, //day high and low

HighBidLowAsk = 64,

Close = 128, //end of day prices

ClosingIndicator = 256, //sent back rp code 7

Settlement = 512,
```
settlement_price: Some(20346.25), settlement_date: Some("20240926"), settlement_price_type: Some("final"),
```
MarketMode = 1024,
```
MarketMode { template_id: 157, symbol: Some("NQ"), exchange: Some("CME"), market_mode: Some("Open"), halt_reason: Some("Group Schedule"), 
trade_date: Some("20240927"), ssboe: None, usecs: None }
```
OpenInterest = 2048,
```
Open Interest (Template ID: 158) from Server: OpenInterest { template_id: 158, symbol: Some("NQ"), exchange: Some("CME"), is_snapshot: Some(true), 
should_clear: None, open_interest: Some(236546), ssboe: None, usecs: None }
```
MarginRate = 4096,
ResponseMarketDataUpdate { template_id: 101, user_msg: [], rp_code: ["7", "no data"] }


HighPriceLimit = 8192,
```
OrderPriceLimits { template_id: 163, symbol: Some("NQ"), exchange: Some("CME"), presence_bits: Some(1), clear_bits: None, is_snapshot: Some(true), 
high_price_limit: Some(21754.25), low_price_limit: None, ssboe: None, usecs: None }
```

LowPriceLimit = 16384,
```
Order Price Limits (Template ID: 163) from Server: OrderPriceLimits { template_id: 163, symbol: Some("NQ"), exchange: Some("CME"), presence_bits: Some(2), 
clear_bits: None, is_snapshot: Some(true), high_price_limit: None, low_price_limit: Some(18938.25), ssboe: None, usecs: None }
```

ProjectedSettlement = 32768,
Market Data Update Response (Template ID: 101) from Server: ResponseMarketDataUpdate { template_id: 101, user_msg: [], rp_code: ["7", "no data"] }


AdjustedClose = 65536,
Market Data Update Response (Template ID: 101) from Server: ResponseMarketDataUpdate { template_id: 101, user_msg: [], rp_code: ["7", "no data"] }
