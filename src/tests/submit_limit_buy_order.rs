use const_decimal::Decimal;

use crate::{
    mock_exchange::MockTransactionAccounting, mock_exchange_linear, prelude::*, test_fee_maker,
    test_fee_taker, trade,
};

#[test]
#[tracing_test::traced_test]
fn submit_limit_buy_order_no_position() {
    let mut exchange = mock_exchange_linear();
    assert!(exchange
        .update_state(
            0.into(),
            &bba!(QuoteCurrency::new(99, 0), QuoteCurrency::new(100, 0))
        )
        .unwrap()
        .is_empty());

    let limit_price = QuoteCurrency::new(98, 0);
    let qty = BaseCurrency::new(5, 0);
    let order = LimitOrder::new(Side::Buy, limit_price, qty).unwrap();
    exchange.submit_limit_order(order.clone()).unwrap();
    assert_eq!(exchange.position(), &Position::Neutral);
    let fee = QuoteCurrency::convert_from(qty, limit_price) * *test_fee_maker().as_ref();
    assert_eq!(
        exchange.user_balances(),
        UserBalances {
            available_wallet_balance: QuoteCurrency::new(510, 0),
            position_margin: QuoteCurrency::new(0, 0),
            order_margin: QuoteCurrency::new(490, 0)
        }
    );

    // Now fill the order
    let ts = 0;
    let meta = ExchangeOrderMeta::new(0.into(), ts.into());
    let mut order = order.into_pending(meta);
    let filled_order = order
        .fill(order.remaining_quantity(), ts.into())
        .expect("Order is fully filled.");
    let expected_order_update = LimitOrderUpdate::FullyFilled(filled_order);
    assert_eq!(
        exchange
            .update_state(
                0.into(),
                &trade!(
                    QuoteCurrency::new(97, 0),
                    BaseCurrency::new(5, 0),
                    Side::Sell
                )
            )
            .unwrap(),
        vec![expected_order_update]
    );
    let bid = QuoteCurrency::new(96, 0);
    let ask = QuoteCurrency::new(99, 0);
    assert!(exchange
        .update_state(0.into(), &bba!(bid, ask))
        .unwrap()
        .is_empty());
    let mut accounting = InMemoryTransactionAccounting::new(QuoteCurrency::new(1000, 0));
    let init_margin_req = Decimal::one();
    assert_eq!(
        exchange.position(),
        &Position::Long(PositionInner::new(
            qty,
            limit_price,
            &mut accounting,
            init_margin_req,
            fee,
        ))
    );
    assert_eq!(
        exchange.user_balances(),
        UserBalances {
            available_wallet_balance: QuoteCurrency::new(510, 0),
            position_margin: QuoteCurrency::new(490, 0),
            order_margin: QuoteCurrency::new(0, 0),
        }
    );
    assert_eq!(
        exchange.position().outstanding_fees(),
        QuoteCurrency::new(98, 3)
    );

    // close the position again with a limit order.
    let order = LimitOrder::new(
        Side::Sell,
        QuoteCurrency::new(98, 0),
        BaseCurrency::new(5, 0),
    )
    .unwrap();
    exchange.submit_limit_order(order.clone()).unwrap();

    assert!(exchange
        .update_state(
            0.into(),
            &bba!(QuoteCurrency::new(96, 0), QuoteCurrency::new(97, 0))
        )
        .unwrap()
        .is_empty());

    let meta = ExchangeOrderMeta::new(1.into(), 0.into());
    let mut order = order.into_pending(meta);
    let filled_order = order
        .fill(order.remaining_quantity(), ts.into())
        .expect("order is filled with this.");
    let expected_order_update = LimitOrderUpdate::FullyFilled(filled_order);
    assert_eq!(
        exchange
            .update_state(
                0.into(),
                &trade!(
                    QuoteCurrency::new(99, 0),
                    BaseCurrency::new(5, 0),
                    Side::Buy
                )
            )
            .unwrap(),
        vec![expected_order_update]
    );
    assert_eq!(exchange.position(), &Position::Neutral);
}

// Test there is a maximum quantity of buy orders the account can post.
#[test]
#[tracing_test::traced_test]
fn submit_limit_buy_order_no_position_max() {
    let mut exchange = mock_exchange_linear();
    assert!(exchange
        .update_state(
            0.into(),
            &bba!(QuoteCurrency::new(100, 0), QuoteCurrency::new(101, 0))
        )
        .unwrap()
        .is_empty());

    let order = LimitOrder::new(
        Side::Buy,
        QuoteCurrency::new(100, 0),
        BaseCurrency::new(5, 0),
    )
    .unwrap();
    exchange.submit_limit_order(order.clone()).unwrap();
    let order = LimitOrder::new(
        Side::Buy,
        QuoteCurrency::new(100, 0),
        BaseCurrency::new(4, 0),
    )
    .unwrap();
    exchange.submit_limit_order(order.clone()).unwrap();
    let order = LimitOrder::new(
        Side::Buy,
        QuoteCurrency::new(100, 0),
        BaseCurrency::new(1, 0),
    )
    .unwrap();
    exchange.submit_limit_order(order.clone()).unwrap();

    let order = LimitOrder::new(
        Side::Sell,
        QuoteCurrency::new(101, 0),
        BaseCurrency::new(5, 0),
    )
    .unwrap();
    exchange.submit_limit_order(order.clone()).unwrap();
    let order = LimitOrder::new(
        Side::Sell,
        QuoteCurrency::new(101, 0),
        BaseCurrency::new(4, 0),
    )
    .unwrap();
    exchange.submit_limit_order(order.clone()).unwrap();
    let order = LimitOrder::new(
        Side::Sell,
        QuoteCurrency::new(101, 0),
        BaseCurrency::new(1, 0),
    )
    .unwrap();
    assert_eq!(
        exchange.submit_limit_order(order.clone()),
        Err(Error::RiskError(RiskError::NotEnoughAvailableBalance))
    );
}

#[test]
#[tracing_test::traced_test]
fn submit_limit_buy_order_with_long() {
    let mut exchange = mock_exchange_linear();
    let mut accounting = MockTransactionAccounting::default();
    let init_margin_req = exchange.config().contract_spec().init_margin_req();
    let bid = QuoteCurrency::new(99, 0);
    let ask = QuoteCurrency::new(100, 0);
    assert!(exchange
        .update_state(0.into(), &bba!(bid, ask))
        .unwrap()
        .is_empty());
    let qty = BaseCurrency::new(9, 0);
    let order = MarketOrder::new(Side::Buy, qty).unwrap();
    exchange.submit_market_order(order).unwrap();

    let fee = QuoteCurrency::convert_from(qty, ask) * *test_fee_taker().as_ref();
    assert_eq!(
        exchange.position().clone(),
        Position::Long(PositionInner::new(
            BaseCurrency::new(9, 0),
            QuoteCurrency::new(100, 0),
            &mut accounting,
            init_margin_req,
            fee,
        )),
    );
    assert_eq!(
        exchange.user_balances(),
        UserBalances {
            available_wallet_balance: QuoteCurrency::new(100, 0),
            position_margin: QuoteCurrency::new(900, 0),
            order_margin: QuoteCurrency::new(0, 0)
        }
    );
    assert_eq!(
        exchange.position().outstanding_fees(),
        QuoteCurrency::new(54, 2)
    );

    assert_eq!(
        exchange
            .update_state(
                0.into(),
                &bba!(QuoteCurrency::new(100, 0), QuoteCurrency::new(101, 0))
            )
            .unwrap(),
        Vec::new()
    );

    // Another buy limit order should not work
    let order = LimitOrder::new(
        Side::Buy,
        QuoteCurrency::new(100, 0),
        BaseCurrency::new(11, 1),
    )
    .unwrap();
    assert_eq!(
        exchange.submit_limit_order(order),
        Err(Error::RiskError(RiskError::NotEnoughAvailableBalance))
    );

    // But sell order should work
    let order = LimitOrder::new(
        Side::Sell,
        QuoteCurrency::new(101, 0),
        BaseCurrency::new(9, 0),
    )
    .unwrap();
    exchange.submit_limit_order(order.clone()).unwrap();

    let ts = 0;
    let meta = ExchangeOrderMeta::new(2.into(), ts.into());
    let mut order = order.into_pending(meta);
    let filled_order = order
        .fill(order.remaining_quantity(), ts.into())
        .expect("order is fully filled");
    let expected_order_update = LimitOrderUpdate::FullyFilled(filled_order);
    assert_eq!(
        exchange
            .update_state(
                0.into(),
                &trade!(
                    QuoteCurrency::new(102, 0),
                    BaseCurrency::new(9, 0),
                    Side::Buy
                )
            )
            .unwrap(),
        vec![expected_order_update]
    );

    assert_eq!(exchange.position(), &Position::Neutral);
}

#[test]
fn submit_limit_buy_order_with_short() {
    let mut exchange = mock_exchange_linear();
    let mut accounting = MockTransactionAccounting::default();
    let init_margin_req = exchange.config().contract_spec().init_margin_req();
    assert!(exchange
        .update_state(
            0.into(),
            &bba!(QuoteCurrency::new(100, 0), QuoteCurrency::new(101, 0))
        )
        .unwrap()
        .is_empty());
    let order = MarketOrder::new(Side::Sell, BaseCurrency::new(9, 0)).unwrap();
    exchange.submit_market_order(order).unwrap();

    let qty = BaseCurrency::new(9, 0);
    let entry_price = QuoteCurrency::new(100, 0);
    let fee = QuoteCurrency::convert_from(qty, entry_price) * *test_fee_taker().as_ref();
    assert_eq!(
        exchange.position().clone(),
        Position::Short(PositionInner::new(
            qty,
            entry_price,
            &mut accounting,
            init_margin_req,
            fee
        ))
    );
    assert_eq!(
        exchange.user_balances(),
        UserBalances {
            available_wallet_balance: QuoteCurrency::new(100, 0),
            position_margin: QuoteCurrency::new(900, 0),
            order_margin: QuoteCurrency::new(0, 0)
        }
    );
    assert_eq!(
        exchange.position().outstanding_fees(),
        QuoteCurrency::new(54, 2)
    );

    // Another sell limit order should not work
    let order = LimitOrder::new(
        Side::Sell,
        QuoteCurrency::new(101, 0),
        BaseCurrency::new(1, 0),
    )
    .unwrap();
    assert_eq!(
        exchange.submit_limit_order(order),
        Err(Error::RiskError(RiskError::NotEnoughAvailableBalance))
    );

    // But buy order should work
    let order = LimitOrder::new(
        Side::Buy,
        QuoteCurrency::new(100, 0),
        BaseCurrency::new(9, 0),
    )
    .unwrap();
    exchange.submit_limit_order(order.clone()).unwrap();

    let ts = 0;
    let meta = ExchangeOrderMeta::new(2.into(), ts.into());
    let mut order = order.into_pending(meta);
    let filled_order = order
        .fill(order.remaining_quantity(), ts.into())
        .expect("Order is filled with this.");
    let expected_order_update = LimitOrderUpdate::FullyFilled(filled_order);
    assert_eq!(
        exchange
            .update_state(
                0.into(),
                &trade!(
                    QuoteCurrency::new(99, 0),
                    BaseCurrency::new(9, 0),
                    Side::Sell
                )
            )
            .unwrap(),
        vec![expected_order_update]
    );

    assert_eq!(exchange.position(), &Position::Neutral);
}

// test rejection if the limit price >= ask
#[test]
fn submit_limit_buy_order_above_ask() {
    let mut exchange = mock_exchange_linear();
    assert_eq!(
        exchange
            .update_state(
                0.into(),
                &bba!(QuoteCurrency::new(99, 0), QuoteCurrency::new(100, 0))
            )
            .unwrap(),
        Vec::new()
    );
    let order = LimitOrder::new(
        Side::Buy,
        QuoteCurrency::new(100, 0),
        BaseCurrency::new(9, 0),
    )
    .unwrap();
    assert_eq!(
        exchange.submit_limit_order(order),
        Err(Error::OrderError(
            OrderError::GoodTillCrossingRejectedOrder {
                limit_price: QuoteCurrency::new(100, 0),
                away_market_quotation_price: QuoteCurrency::new(100, 0)
            }
        ))
    );
}

// With a short position open, be able to open a long position of equal size using a limit order
// TODO: this requires a change in the `IsolatedMarginRiskEngine`
#[test]
fn submit_limit_buy_order_turnaround_short() {
    // let mut exchange = mock_exchange_base();
    // assert_eq!(
    //     exchange
    //         .update_state(0, bba!(QuoteCurrency::new(100), QuoteCurrency::new(101)))
    //         .unwrap(),
    //     vec![]
    // );
    // let order = Order::market(Side::Sell, BaseCurrency::new(9)).unwrap();
    // exchange.submit_limit_order(order).unwrap();

    // let order = LimitOrder::new(Side::Buy, QuoteCurrency::new(100), BaseCurrency::new(18)).unwrap();
    // exchange.submit_limit_order(order.clone()).unwrap();

    // // Execute the limit buy order
    // assert_eq!(
    //     exchange
    //         .update_state(0, bba!(QuoteCurrency::new(98), QuoteCurrency::new(99)))
    //         .unwrap(),
    //     vec![order]
    // );
    // assert_eq!(
    //     exchange.account().position(),
    //     &Position {
    //         size: BaseCurrency::new(9),
    //         entry_price: QuoteCurrency::new(100),
    //         position_margin: QuoteCurrency::new(900),
    //         leverage: leverage!(1),
    //     }
    // );
}
