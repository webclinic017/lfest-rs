use crate::{mock_exchange_base, prelude::*};

fn submit_limit_sell_order_no_position() {
    let mut exchange = mock_exchange_base();
    assert_eq!(
        exchange
            .update_state(0, bba!(quote!(99), quote!(100)))
            .unwrap(),
        vec![]
    );

    let mut order = Order::limit(Side::Sell, quote!(100), base!(9)).unwrap();
    exchange.submit_order(order.clone()).unwrap();

    assert_eq!(
        exchange.account().position,
        Position {
            size: base!(0),
            entry_price: quote!(0),
            position_margin: quote!(0),
            leverage: leverage!(1),
        }
    );

    // Now fill the order
    assert_eq!(
        exchange
            .update_state(0, bba!(quote!(101), quote!(102)))
            .unwrap(),
        vec![order]
    );
    assert_eq!(
        exchange.account().position,
        Position {
            size: base!(9),
            entry_price: quote!(100),
            position_margin: quote!(900),
            leverage: leverage!(1),
        }
    );
    let fee = quote!(0.1);
    assert_eq!(exchange.account().wallet_balance, quote!(1000) - fee);
    assert_eq!(exchange.account().available_balance(), quote!(900) - fee);

    // close the position again
    let mut order = Order::limit(Side::Buy, quote!(100), base!(9)).unwrap();
    exchange.submit_order(order.clone()).unwrap();

    order.set_id(1);
    assert_eq!(
        exchange
            .update_state(0, bba!(quote!(99), quote!(100)))
            .unwrap(),
        vec![order]
    );
    assert_eq!(
        exchange.account().position,
        Position {
            size: base!(0),
            entry_price: quote!(100),
            position_margin: quote!(0),
            leverage: leverage!(1),
        }
    );
    assert_eq!(exchange.account().wallet_balance, quote!(1000) - fee - fee);
    assert_eq!(
        exchange.account().available_balance(),
        quote!(1000) - fee - fee
    );
}
