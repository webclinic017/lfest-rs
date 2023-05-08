use hashbrown::HashMap;

use crate::{
    exchange::EXPECT_LIMIT_PRICE,
    position::Position,
    types::{Currency, Error, Leverage, MarginCurrency, Order, OrderType, Result, Side},
    utils::{max, min},
};

#[derive(Debug, Clone)]
/// The users account
/// Generic over:
/// S: The `Currency` representing the order quantity
pub struct Account<M>
where
    M: Currency + MarginCurrency,
{
    pub(crate) wallet_balance: M,
    pub(crate) position: Position<M>,
    // Maps the order `id` to the actual `Order`.
    pub(crate) active_limit_orders: HashMap<u64, Order<M::PairedCurrency>>,
    // Maps the `user_order_id` to the internal order nonce
    pub(crate) lookup_order_nonce_from_user_order_id: HashMap<u64, u64>,
    pub(crate) next_order_id: u64,
}

impl<M> Account<M>
where
    M: Currency + MarginCurrency,
{
    /// Create a new [`Account`] instance.
    pub(crate) fn new(starting_balance: M, leverage: Leverage) -> Self {
        let position = Position::new(leverage);

        Self {
            wallet_balance: starting_balance,
            position,
            active_limit_orders: HashMap::new(),
            lookup_order_nonce_from_user_order_id: HashMap::new(),
            next_order_id: 0,
        }
    }

    /// Return a reference to the accounts position.
    #[inline(always)]
    pub fn position(&self) -> &Position<M> {
        &self.position
    }

    /// Return the current wallet balance of the account.
    #[inline(always)]
    pub fn wallet_balance(&self) -> M {
        self.wallet_balance
    }

    /// Return the available balance of the `Account`
    #[inline(always)]
    pub fn available_balance(&self) -> M {
        // TODO - order_margin
        warn!("order_margin not included in `available_balance` calculation!");
        self.wallet_balance - self.position.position_margin
    }

    /// Allows the user to update their desired leverage.
    /// This will deposit or release variation margin from the position if any.
    ///
    /// # Returns:
    /// If Err, the account is unable to provide enough variation margin for the desired leverage.
    pub fn update_desired_leverage(&mut self, leverage: Leverage) -> Result<()> {
        todo!()
    }

    /// Cancel an active order based on the user_order_id of an Order
    ///
    /// # Returns:
    /// the cancelled order if successfull, error when the `user_order_id` is
    /// not found
    pub fn cancel_order_by_user_id(
        &mut self,
        user_order_id: u64,
    ) -> Result<Order<M::PairedCurrency>> {
        debug!("cancel_order_by_user_id: user_order_id: {}", user_order_id);
        let id: u64 = match self
            .lookup_order_nonce_from_user_order_id
            .remove(&user_order_id)
        {
            None => return Err(Error::UserOrderIdNotFound),
            Some(id) => id,
        };
        self.cancel_order(id)
    }

    /// Append a new limit order as active order
    pub(crate) fn append_limit_order(&mut self, order: Order<M::PairedCurrency>) {
        debug_assert!(matches!(order.order_type(), OrderType::Limit));

        debug!("append_limit_order: order: {:?}", order);

        // self.account_tracker.log_limit_order_submission();
        let order_id = order.id();
        let user_order_id = *order.user_order_id();
        match self.active_limit_orders.insert(order_id, order) {
            None => {}
            Some(_) => {
                error!(
                    "there already was an order with this id in active_limit_orders. \
            This should not happen as order id should be incrementing"
                );
                debug_assert!(false)
            }
        };
        match user_order_id {
            None => {}
            Some(user_order_id) => {
                self.lookup_order_nonce_from_user_order_id
                    .insert(user_order_id, order_id);
            }
        };
    }

    /// Cancel an active order
    /// returns Some order if successful with given order_id
    pub fn cancel_order(&mut self, order_id: u64) -> Result<Order<M::PairedCurrency>> {
        debug!("cancel_order: {}", order_id);
        let removed_order = self
            .active_limit_orders
            .remove(&order_id)
            .ok_or(Error::OrderIdNotFound)?;

        // self.account_tracker.log_limit_order_cancellation();

        Ok(removed_order)
    }

    /// Removes an executed limit order from the list of active ones
    pub(crate) fn remove_executed_order_from_active(&mut self, order_id: u64) {
        let order = self
            .active_limit_orders
            .remove(&order_id)
            .expect("The order must have been active; qed");
        if let Some(user_order_id) = order.user_order_id() {
            self.lookup_order_nonce_from_user_order_id
                .remove(user_order_id);
        }
    }

    /// Compute the current order margin requirement.
    fn compute_order_margin(&self) -> M {
        let mut open_buy_quantity: M::PairedCurrency = self
            .active_limit_orders
            .values()
            .filter(|order| matches!(order.side(), Side::Buy))
            .map(|order| order.quantity())
            .fold(M::PairedCurrency::new_zero(), |acc, x| acc + x);
        let mut open_sell_quantity: M::PairedCurrency = self
            .active_limit_orders
            .values()
            .filter(|order| matches!(order.side(), Side::Sell))
            .map(|order| order.quantity())
            .fold(M::PairedCurrency::new_zero(), |acc, x| acc + x);

        // Offset against the open position size.
        if self.position.size() > M::PairedCurrency::new_zero() {
            open_sell_quantity = open_sell_quantity - self.position.size();
        } else {
            open_buy_quantity = open_buy_quantity - self.position.size().abs();
        }

        let mut buy_notional_value_sum = self
            .active_limit_orders
            .values()
            .filter(|order| matches!(order.side(), Side::Buy))
            .map(|order| {
                order
                    .quantity()
                    .convert(order.limit_price().expect(EXPECT_LIMIT_PRICE))
            })
            .fold(M::new_zero(), |acc, x| acc + x);

        // Offset the limit order cost by a potential short position
        buy_notional_value_sum = max(
            buy_notional_value_sum
                - min(self.position.size(), M::PairedCurrency::new_zero())
                    .abs()
                    .convert(self.position.entry_price),
            M::new_zero(),
        );

        // The sell orders dominate
        let mut sell_notional_value_sum = self
            .active_limit_orders
            .values()
            .filter(|order| matches!(order.side(), Side::Sell))
            .map(|order| {
                order
                    .quantity()
                    .convert(order.limit_price().expect(EXPECT_LIMIT_PRICE))
            })
            .fold(M::new_zero(), |acc, x| acc + x);

        // Offset the limit order cost by a potential long position
        sell_notional_value_sum = max(
            M::new_zero(),
            sell_notional_value_sum
                - max(M::PairedCurrency::new_zero(), self.position.size())
                    .convert(self.position.entry_price),
        );

        max(buy_notional_value_sum, sell_notional_value_sum) / self.position.leverage
    }

    #[inline(always)]
    fn next_order_id(&mut self) -> u64 {
        self.next_order_id += 1;
        self.next_order_id - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    #[test]
    fn account_order_margin_no_position() {
        let mut account = Account::new(quote!(1000), leverage!(1));

        assert_eq!(account.compute_order_margin(), quote!(0));

        let mut order = Order::limit(Side::Buy, quote!(90), base!(1)).unwrap();
        order.set_id(0);
        account.append_limit_order(order);
        assert_eq!(account.compute_order_margin(), quote!(90));

        let mut order = Order::limit(Side::Sell, quote!(100), base!(1)).unwrap();
        order.set_id(1);
        account.append_limit_order(order);
        assert_eq!(account.compute_order_margin(), quote!(100));

        let mut order = Order::limit(Side::Sell, quote!(120), base!(1)).unwrap();
        order.set_id(2);
        account.append_limit_order(order);
        assert_eq!(account.compute_order_margin(), quote!(220));
    }

    #[test]
    fn account_order_margin_with_long() {
        let _ = pretty_env_logger::try_init();

        let mut account = Account::new(quote!(1000), leverage!(1));
        account.position = Position {
            size: base!(1),
            entry_price: quote!(100),
            position_margin: quote!(100),
            leverage: leverage!(1),
        };
        assert_eq!(account.compute_order_margin(), quote!(0));

        let mut order = Order::limit(Side::Buy, quote!(90), base!(1)).unwrap();
        order.set_id(0);
        account.append_limit_order(order);
        assert_eq!(account.compute_order_margin(), quote!(90));

        let mut order = Order::limit(Side::Sell, quote!(100), base!(1)).unwrap();
        order.set_id(1);
        account.append_limit_order(order);
        assert_eq!(account.compute_order_margin(), quote!(90));

        let mut order = Order::limit(Side::Sell, quote!(120), base!(1)).unwrap();
        order.set_id(2);
        account.append_limit_order(order);
        assert_eq!(account.compute_order_margin(), quote!(120));

        let mut order = Order::limit(Side::Buy, quote!(95), base!(1)).unwrap();
        order.set_id(3);
        account.append_limit_order(order);
        assert_eq!(account.compute_order_margin(), quote!(185));
    }

    #[test]
    fn account_order_margin_with_short() {
        let _ = pretty_env_logger::try_init();

        let mut account = Account::new(quote!(1000), leverage!(1));
        account.position = Position {
            size: base!(-1),
            entry_price: quote!(100),
            position_margin: quote!(100),
            leverage: leverage!(1),
        };
        assert_eq!(account.compute_order_margin(), quote!(0));

        let mut order = Order::limit(Side::Buy, quote!(90), base!(1)).unwrap();
        order.set_id(0);
        account.append_limit_order(order);
        assert_eq!(account.compute_order_margin(), quote!(0));

        let mut order = Order::limit(Side::Sell, quote!(100), base!(1)).unwrap();
        order.set_id(1);
        account.append_limit_order(order);
        assert_eq!(account.compute_order_margin(), quote!(100));

        let mut order = Order::limit(Side::Sell, quote!(120), base!(1)).unwrap();
        order.set_id(2);
        account.append_limit_order(order);
        assert_eq!(account.compute_order_margin(), quote!(220));

        let mut order = Order::limit(Side::Buy, quote!(95), base!(1)).unwrap();
        order.set_id(3);
        account.append_limit_order(order);
        assert_eq!(account.compute_order_margin(), quote!(220));
    }
}
