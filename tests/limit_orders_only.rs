//! Test if a pure limit order strategy works correctly

use lfest::*;
use log::*;

#[test]
fn limit_orders_only() {
    if let Err(_) = pretty_env_logger::try_init() {}

    let config = Config::new(
        Fee(0.0002),
        Fee(0.0006),
        quote!(1000.0),
        1.0,
        FuturesTypes::Linear,
        String::new(),
        true,
    )
    .unwrap();

    let acc_tracker = NoAccountTracker::default();
    let mut exchange = Exchange::new(acc_tracker, config);

    let (exec_orders, liq) = exchange.update_state(
        0,
        MarketUpdate::Bba {
            bid: quote!(100.0),
            ask: quote!(100.1),
        },
    );
    assert!(!liq);
    assert_eq!(exec_orders.len(), 0);

    let o = Order::limit(Side::Buy, quote!(100.0), base!(9.9)).unwrap();
    exchange.submit_order(o).unwrap();
    assert_eq!(exchange.account().margin().order_margin(), quote!(990.198));
    assert_eq!(exchange.account().margin().available_balance().into_rounded(3), quote!(9.802));

    let (exec_orders, liq) = exchange.update_state(
        1,
        MarketUpdate::Bba {
            bid: quote!(99.9),
            ask: quote!(100.0),
        },
    );
    assert!(!liq);
    assert_eq!(exec_orders.len(), 1);
    debug!("exec_orders: {:?}", exec_orders);

    assert_eq!(exchange.account().position().size(), base!(9.9));
    assert_eq!(exchange.account().position().entry_price(), quote!(100.0));
    // TODO: upnl uses mid price but should use the expected fill price, meaning it
    // should be 0.99 not 0.495 assert_eq!(exchange.account().position().
    // unrealized_pnl(), 0.0);

    assert_eq!(exchange.account().margin().wallet_balance(), quote!(999.802));
    assert_eq!(exchange.account().margin().position_margin(), quote!(990.0));
    assert_eq!(exchange.account().margin().order_margin(), quote!(0.0));
    assert_eq!(exchange.account().margin().available_balance().into_rounded(3), quote!(9.802));

    let o = Order::limit(Side::Sell, quote!(105.1), base!(9.9)).unwrap();
    exchange.submit_order(o).unwrap();
    assert_eq!(exchange.account().margin().order_margin(), quote!(0.0));

    let (exec_orders, liq) = exchange.update_state(
        2,
        MarketUpdate::Bba {
            bid: quote!(106.0),
            ask: quote!(106.1),
        },
    );
    assert!(!liq);
    assert!(!exec_orders.is_empty());

    assert_eq!(exchange.account().position().size(), base!(0.0));
    assert_eq!(exchange.account().margin().wallet_balance().into_rounded(6), quote!(1050.083902));
    assert_eq!(exchange.account().margin().position_margin(), quote!(0.0));
    assert_eq!(exchange.account().margin().order_margin(), quote!(0.0));
    assert_eq!(
        exchange.account().margin().available_balance().into_rounded(6),
        quote!(1050.083902)
    );
}

#[test]
fn limit_orders_2() {
    if let Err(_) = pretty_env_logger::try_init() {}

    let config = Config::new(
        Fee(0.0002),
        Fee(0.0006),
        quote!(100.0),
        1.0,
        FuturesTypes::Linear,
        String::new(),
        true,
    )
    .unwrap();

    let acc_tracker = NoAccountTracker::default();
    let mut exchange = Exchange::new(acc_tracker, config);

    let (exec_orders, liq) = exchange.update_state(
        0,
        MarketUpdate::Bba {
            bid: quote!(100.0),
            ask: quote!(100.1),
        },
    );
    assert!(!liq);
    assert!(exec_orders.is_empty());

    let o = Order::limit(Side::Sell, quote!(100.1), base!(0.75)).unwrap();
    exchange.submit_order(o).unwrap();

    let o = Order::limit(Side::Buy, quote!(100.0), base!(0.5)).unwrap();
    exchange.submit_order(o).unwrap();

    let (exec_orders, liq) = exchange.update_state(
        1,
        MarketUpdate::Bba {
            bid: quote!(99.0),
            ask: quote!(99.1),
        },
    );
    assert!(!liq);
    assert_eq!(exec_orders.len(), 1);
}
