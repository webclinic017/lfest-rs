//! Test file for the linear futures mode of the exchange

use lfest::{mock_exchange_base, prelude::*};

#[test]
#[tracing_test::traced_test]
fn lin_long_market_win_full() {
    let mut exchange = mock_exchange_base();
    let _ = exchange
        .update_state(
            0,
            MarketUpdate::Bba {
                bid: quote!(99.0),
                ask: quote!(100.0),
            },
        )
        .unwrap();

    exchange
        .submit_market_order(MarketOrder::new(Side::Buy, base!(5.0)).unwrap())
        .unwrap();
    let _ = exchange
        .update_state(
            0,
            MarketUpdate::Bba {
                bid: quote!(100.0),
                ask: quote!(101.0),
            },
        )
        .unwrap();

    assert_eq!(exchange.account().position().size(), base!(5.0));
    assert_eq!(exchange.account().position().entry_price(), quote!(100.0));
    assert_eq!(
        exchange
            .account()
            .position()
            .unrealized_pnl(quote!(100), quote!(101)),
        quote!(0.0)
    );
    assert_eq!(exchange.account().wallet_balance(), quote!(999.7));
    assert_eq!(exchange.account().position().margin(), quote!(500.0));
    assert_eq!(exchange.account().available_balance(), quote!(499.7));

    let _ = exchange
        .update_state(
            0,
            MarketUpdate::Bba {
                bid: quote!(200),
                ask: quote!(201),
            },
        )
        .unwrap();
    assert_eq!(
        exchange
            .account()
            .position()
            .unrealized_pnl(quote!(200), quote!(201)),
        quote!(500.0)
    );

    exchange
        .submit_market_order(MarketOrder::new(Side::Sell, base!(5.0)).unwrap())
        .unwrap();

    assert_eq!(exchange.account().position().size(), base!(0.0));
    assert_eq!(exchange.account().position().entry_price(), quote!(100.0));
    assert_eq!(
        exchange
            .account()
            .position()
            .unrealized_pnl(quote!(200), quote!(201)),
        quote!(0.0)
    );
    assert_eq!(exchange.account().wallet_balance(), quote!(1499.1));
    assert_eq!(exchange.account().position().margin(), quote!(0.0));
    assert_eq!(exchange.account().available_balance(), quote!(1499.1));
}
