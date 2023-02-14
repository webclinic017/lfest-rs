use std::fmt::Display;

use crate::{cornish_fisher::cornish_fisher_value_at_risk, AccountTracker, FuturesTypes, Side};

const DAILY_NS: u64 = 86_400_000_000_000;
const HOURLY_NS: u64 = 3_600_000_000_000;

/// Defines the possible sources of returns to use
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ReturnsSource {
    /// Daily sampled returns
    Daily,
    /// Hourly sampled returns
    Hourly,
    /// Tick-by-tick sampled returns
    TickByTick,
}

/// Keep track of many possible Account performance statistics
/// This can be quite memory intensive, easily reaching beyond 10GB
/// if using tick-by-tick data due to the storage of many returns
#[derive(Debug, Clone)]
pub struct FullAccountTracker {
    wallet_balance_last: f64,  // last wallet balance recording
    wallet_balance_start: f64, // wallet balance at start
    wallet_balance_high: f64,  // maximum wallet balance observed
    futures_type: FuturesTypes,
    total_rpnl: f64,
    upnl: f64,
    num_trades: i64,
    num_buys: i64,
    num_wins: usize,
    num_losses: usize,
    num_submitted_limit_orders: usize,
    num_cancelled_limit_orders: usize,
    num_filled_limit_orders: usize,
    num_trading_opportunities: usize,
    total_turnover: f64,
    max_drawdown_wallet_balance: f64,
    max_drawdown_total: f64,
    // historical daily absolute returns
    hist_returns_daily_acc: Vec<f64>,
    hist_returns_daily_bnh: Vec<f64>,
    // historical hourly absolute returns
    hist_returns_hourly_acc: Vec<f64>,
    hist_returns_hourly_bnh: Vec<f64>,
    // historical tick by tick absolute returns
    // TODO: if these tick-by-tick returns vectors get too large, disable it in live mode
    hist_returns_tick_acc: Vec<f64>,
    hist_returns_tick_bnh: Vec<f64>,
    // historical daily logarithmic returns
    hist_ln_returns_daily_acc: Vec<f64>,
    hist_ln_returns_daily_bnh: Vec<f64>,
    // historical hourly logarithmic returns
    hist_ln_returns_hourly_acc: Vec<f64>,
    hist_ln_returns_hourly_bnh: Vec<f64>,
    // historical tick by tick logarithmic returns
    hist_ln_returns_tick_acc: Vec<f64>,
    hist_ln_returns_tick_bnh: Vec<f64>,
    // timestamps for when to trigger the next pnl snapshots
    next_daily_trigger_ts: u64,
    next_hourly_trigger_ts: u64,
    last_daily_pnl: f64,
    last_hourly_pnl: f64,
    last_tick_pnl: f64,
    cumulative_fees: f64,
    total_profit: f64,
    total_loss: f64,
    price_first: f64,
    price_last: f64,
    price_a_day_ago: f64,
    price_an_hour_ago: f64,
    price_a_tick_ago: f64,
    ts_first: u64,
    ts_last: u64,
}

impl FullAccountTracker {
    #[must_use]
    #[inline]
    /// Create a new AccTracker struct
    pub fn new(starting_wb: f64, futures_type: FuturesTypes) -> Self {
        FullAccountTracker {
            wallet_balance_last: starting_wb,
            wallet_balance_start: starting_wb,
            wallet_balance_high: starting_wb,
            futures_type,
            total_rpnl: 0.0,
            upnl: 0.0,
            num_trades: 0,
            num_buys: 0,
            num_wins: 0,
            num_losses: 0,
            num_submitted_limit_orders: 0,
            num_cancelled_limit_orders: 0,
            num_filled_limit_orders: 0,
            num_trading_opportunities: 0,
            total_turnover: 0.0,
            max_drawdown_wallet_balance: 0.0,
            max_drawdown_total: 0.0,
            hist_returns_daily_acc: vec![],
            hist_returns_daily_bnh: vec![],
            hist_returns_hourly_acc: vec![],
            hist_returns_hourly_bnh: vec![],
            hist_returns_tick_acc: vec![],
            hist_returns_tick_bnh: vec![],
            hist_ln_returns_daily_acc: vec![],
            hist_ln_returns_daily_bnh: vec![],
            hist_ln_returns_hourly_acc: vec![],
            hist_ln_returns_hourly_bnh: vec![],
            hist_ln_returns_tick_acc: vec![],
            hist_ln_returns_tick_bnh: vec![],
            next_daily_trigger_ts: 0,
            next_hourly_trigger_ts: 0,
            last_daily_pnl: 0.0,
            last_hourly_pnl: 0.0,
            last_tick_pnl: 0.0,
            cumulative_fees: 0.0,
            total_profit: 0.0,
            total_loss: 0.0,
            price_first: 0.0,
            price_last: 0.0,
            price_a_day_ago: 0.0,
            price_an_hour_ago: 0.0,
            price_a_tick_ago: 0.0,
            ts_first: 0,
            ts_last: 0,
        }
    }

    /// Vector of absolute returns the account has generated, including
    /// unrealized pnl # Parameters
    /// source: the sampling interval of pnl snapshots
    #[inline(always)]
    pub fn absolute_returns(&self, source: &ReturnsSource) -> &Vec<f64> {
        match source {
            ReturnsSource::Daily => &self.hist_returns_daily_acc,
            ReturnsSource::Hourly => &self.hist_returns_hourly_acc,
            ReturnsSource::TickByTick => &self.hist_returns_tick_acc,
        }
    }

    /// Vector of natural logarithmic returns the account has generated,
    /// including unrealized pnl # Parameters
    /// source: the sampling interval of pnl snapshots
    #[inline(always)]
    pub fn ln_returns(&self, source: &ReturnsSource) -> &Vec<f64> {
        match source {
            ReturnsSource::Daily => &self.hist_ln_returns_daily_acc,
            ReturnsSource::Hourly => &self.hist_ln_returns_hourly_acc,
            ReturnsSource::TickByTick => &self.hist_ln_returns_tick_acc,
        }
    }

    /// Ratio of cumulative trade profit over cumulative trade loss
    #[inline(always)]
    pub fn profit_loss_ratio(&self) -> f64 {
        self.total_profit / self.total_loss
    }

    /// Cumulative fees paid to the exchange
    #[inline(always)]
    pub fn cumulative_fees(&self) -> f64 {
        self.cumulative_fees
    }

    /// Would be return of buy and hold strategy
    #[inline(always)]
    pub fn buy_and_hold_return(&self) -> f64 {
        let qty = match self.futures_type {
            FuturesTypes::Linear => self.wallet_balance_start / self.price_first,
            FuturesTypes::Inverse => self.wallet_balance_start * self.price_first,
        };
        self.futures_type.pnl(self.price_first, self.price_last, qty)
    }

    /// Would be return of sell and hold strategy
    #[inline(always)]
    pub fn sell_and_hold_return(&self) -> f64 {
        let qty = match self.futures_type {
            FuturesTypes::Linear => self.wallet_balance_start / self.price_first,
            FuturesTypes::Inverse => self.wallet_balance_start * self.price_first,
        };
        self.futures_type.pnl(self.price_first, self.price_last, -qty)
    }

    /// Return the sharpe ratio using the selected returns as source
    /// # Parameters:
    /// returns_source: the sampling interval of pnl snapshots
    /// risk_free_is_buy_and_hold: if true, it will use the market returns as
    /// the risk-free comparison     else risk-free rate is zero
    pub fn sharpe(&self, returns_source: ReturnsSource, risk_free_is_buy_and_hold: bool) -> f64 {
        let rets_acc = match returns_source {
            ReturnsSource::Daily => &self.hist_returns_daily_acc,
            ReturnsSource::Hourly => &self.hist_returns_hourly_acc,
            ReturnsSource::TickByTick => &self.hist_returns_tick_acc,
        };
        if risk_free_is_buy_and_hold {
            let rets_bnh = match returns_source {
                ReturnsSource::Daily => &self.hist_returns_daily_bnh,
                ReturnsSource::Hourly => &self.hist_returns_hourly_bnh,
                ReturnsSource::TickByTick => &self.hist_returns_tick_bnh,
            };
            let n: f64 = rets_acc.len() as f64;
            // compute the difference of returns of account and market
            let diff_returns: Vec<f64> =
                rets_acc.iter().zip(rets_bnh).map(|(a, b)| *a - *b).collect();
            let avg = diff_returns.iter().sum::<f64>() / n;
            let variance = diff_returns.iter().map(|v| (*v - avg).powi(2)).sum::<f64>() / n;
            let std_dev = variance.sqrt();

            (self.total_rpnl - self.buy_and_hold_return()) / std_dev
        } else {
            let n = rets_acc.len() as f64;
            let avg = rets_acc.iter().sum::<f64>() / n;
            let var = rets_acc.iter().map(|v| (*v - avg).powi(2)).sum::<f64>() / n;
            let std_dev = var.sqrt();

            self.total_rpnl / std_dev
        }
    }

    /// Return the Sortino ratio based on daily returns data
    /// # Parameters:
    /// returns_source: the sampling interval of pnl snapshots
    /// risk_free_is_buy_and_hold: if true, it will use the market returns as
    /// the risk-free comparison     else risk-free rate is zero
    pub fn sortino(&self, returns_source: ReturnsSource, risk_free_is_buy_and_hold: bool) -> f64 {
        let rets_acc = match returns_source {
            ReturnsSource::Daily => &self.hist_returns_daily_acc,
            ReturnsSource::Hourly => &self.hist_returns_hourly_acc,
            ReturnsSource::TickByTick => &self.hist_returns_tick_acc,
        };
        if risk_free_is_buy_and_hold {
            let rets_bnh = match returns_source {
                ReturnsSource::Daily => &self.hist_returns_daily_bnh,
                ReturnsSource::Hourly => &self.hist_returns_hourly_bnh,
                ReturnsSource::TickByTick => &self.hist_returns_tick_bnh,
            };
            // compute the difference of returns of account and market
            let diff_returns: Vec<f64> =
                rets_acc.iter().zip(rets_bnh).map(|(a, b)| *a - *b).filter(|v| *v < 0.0).collect();
            let n: f64 = diff_returns.len() as f64;
            let avg = diff_returns.iter().sum::<f64>() / n;
            let variance = diff_returns.iter().map(|v| (*v - avg).powi(2)).sum::<f64>() / n;
            let std_dev = variance.sqrt();

            (self.total_rpnl - self.buy_and_hold_return()) / std_dev
        } else {
            let downside_rets: Vec<f64> = rets_acc.iter().copied().filter(|v| *v < 0.0).collect();
            let n = downside_rets.len() as f64;
            let avg = downside_rets.iter().sum::<f64>() / n;
            let var = downside_rets.iter().map(|v| (*v - avg).powi(2)).sum::<f64>() / n;
            let std_dev = var.sqrt();

            self.total_rpnl / std_dev
        }
    }

    /// Calculate the value at risk using the percentile method on daily returns
    /// multiplied by starting wallet balance The time horizon N is assumed
    /// to be 1 The literature says if you want a larger N, just multiply by
    /// N.sqrt(), which assumes standard normal distribution # Arguments
    /// returns_source: the sampling interval of pnl snapshots
    /// percentile: value between [0.0, 1.0], smaller value will return more
    /// worst case results
    #[inline]
    pub fn historical_value_at_risk(&self, returns_source: ReturnsSource, percentile: f64) -> f64 {
        let mut rets = match returns_source {
            ReturnsSource::Daily => self.hist_ln_returns_daily_acc.clone(),
            ReturnsSource::Hourly => self.hist_ln_returns_hourly_acc.clone(),
            ReturnsSource::TickByTick => self.hist_ln_returns_tick_acc.clone(),
        };
        rets.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let idx = (rets.len() as f64 * percentile) as usize;
        match rets.get(idx) {
            Some(r) => self.wallet_balance_start - (self.wallet_balance_start * r.exp()),
            None => 0.0,
        }
    }

    /// Calculate the historical value at risk from n consequtive hourly return
    /// values, This should have better statistical properties compared to
    /// using daily returns due to having more samples. Set n to 24 for
    /// daily value at risk, but with 24x more samples from which to take the
    /// percentile, giving a more accurate VaR
    /// # Parameters:
    /// n: number of hourly returns to use
    /// percentile: value between [0.0, 1.0], smaller value will return more
    /// worst case results
    pub fn historical_value_at_risk_from_n_hourly_returns(&self, n: usize, percentile: f64) -> f64 {
        let rets = &self.hist_ln_returns_hourly_acc;
        if rets.len() < n {
            debug!("not enough hourly returns to compute VaR for n={}", n);
            return 0.0;
        }
        let mut ret_streaks = Vec::with_capacity(rets.len() - n);
        for i in n..rets.len() {
            let mut r = 1.0;
            for ret in rets.iter().take(i).skip(i - n) {
                r *= ret.exp();
            }
            ret_streaks.push(r);
        }

        ret_streaks.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let idx = (ret_streaks.len() as f64 * percentile) as usize;
        match ret_streaks.get(idx) {
            Some(r) => self.wallet_balance_start - (self.wallet_balance_start * r),
            None => 0.0,
        }
    }

    /// Calculate the cornish fisher value at risk based on daily returns of the
    /// account # Arguments
    /// returns_source: the sampling interval of pnl snapshots
    /// percentile: in range [0.0, 1.0], usually something like 0.01 or 0.05
    #[inline]
    pub fn cornish_fisher_value_at_risk(
        &self,
        returns_source: ReturnsSource,
        percentile: f64,
    ) -> f64 {
        let rets = match returns_source {
            ReturnsSource::Daily => &self.hist_ln_returns_daily_acc,
            ReturnsSource::Hourly => &self.hist_ln_returns_hourly_acc,
            ReturnsSource::TickByTick => &self.hist_ln_returns_tick_acc,
        };
        cornish_fisher_value_at_risk(rets, self.wallet_balance_start, percentile).2
    }

    /// Calculate the corni fisher value at risk from n consequtive hourly
    /// return values This should have better statistical properties
    /// compared to using daily returns due to having more samples. Set n to
    /// 24 for daily value at risk, but with 24x more samples from which to take
    /// the percentile, giving a more accurate VaR
    /// # Parameters:
    /// n: number of hourly returns to use
    /// percentile: value between [0.0, 1.0], smaller value will return more
    /// worst case results
    pub fn cornish_fisher_value_at_risk_from_n_hourly_returns(
        &self,
        n: usize,
        percentile: f64,
    ) -> f64 {
        let rets = &self.hist_ln_returns_hourly_acc;
        if rets.len() < n {
            debug!("not enough hourly returns to compute CF-VaR for n={}", n);
            return 0.0;
        }
        let mut ret_streaks = Vec::with_capacity(rets.len() - n);
        for i in n..rets.len() {
            let mut r = 1.0;
            for ret in rets.iter().take(i).skip(i - n) {
                r *= ret.exp();
            }
            ret_streaks.push(r);
        }

        self.wallet_balance_start
            - (self.wallet_balance_start
                * cornish_fisher_value_at_risk(&ret_streaks, self.wallet_balance_start, percentile)
                    .1)
    }

    /// Return the number of trading days
    #[inline(always)]
    pub fn num_trading_days(&self) -> u64 {
        (self.ts_last - self.ts_first) / DAILY_NS
    }

    /// Also called discriminant-ratio, which focuses on the added value of the
    /// algorithm It uses the Cornish-Fish Value at Risk (CF-VaR)
    /// It better captures the risk of the asset as it is not limited by the
    /// assumption of a gaussian distribution It it time-insensitive
    /// from: https://papers.ssrn.com/sol3/papers.cfm?abstract_id=3927058
    /// # Parameters
    /// returns_source: the sampling interval of pnl snapshots
    pub fn d_ratio(&self, returns_source: ReturnsSource) -> f64 {
        let rets_acc = match returns_source {
            ReturnsSource::Daily => &self.hist_ln_returns_daily_acc,
            ReturnsSource::Hourly => &self.hist_ln_returns_hourly_acc,
            ReturnsSource::TickByTick => &self.hist_ln_returns_tick_acc,
        };
        let rets_bnh = match returns_source {
            ReturnsSource::Daily => &self.hist_ln_returns_daily_bnh,
            ReturnsSource::Hourly => &self.hist_ln_returns_hourly_bnh,
            ReturnsSource::TickByTick => &self.hist_ln_returns_tick_bnh,
        };

        let cf_var_bnh = cornish_fisher_value_at_risk(rets_bnh, self.wallet_balance_start, 0.01).1;
        let cf_var_acc = cornish_fisher_value_at_risk(rets_acc, self.wallet_balance_start, 0.01).1;

        let num_trading_days = self.num_trading_days() as f64;

        // compute annualized returns
        let roi_acc =
            rets_acc.iter().fold(1.0, |acc, x| acc * x.exp()).powf(365.0 / num_trading_days);
        let roi_bnh =
            rets_bnh.iter().fold(1.0, |acc, x| acc * x.exp()).powf(365.0 / num_trading_days);

        let rtv_acc = roi_acc / cf_var_acc;
        let rtv_bnh = roi_bnh / cf_var_bnh;
        debug!(
            "roi_acc: {:.2}, roi_bnh: {:.2}, cf_var_bnh: {:.8}, cf_var_acc: {:.8}, rtv_acc: {}, rtv_bnh: {}",
            roi_acc, roi_bnh, cf_var_bnh, cf_var_acc, rtv_acc, rtv_bnh,
        );

        (1.0 + (roi_acc - roi_bnh) / roi_bnh.abs()) * (cf_var_bnh / cf_var_acc)
    }

    /// Annualized return on investment as a factor, e.g.: 100% -> 2x
    pub fn annualized_roi(&self) -> f64 {
        (1.0 + (self.total_rpnl / self.wallet_balance_start))
            .powf(365.0 / self.num_trading_days() as f64)
    }

    /// Maximum drawdown of the wallet balance
    #[inline(always)]
    pub fn max_drawdown_wallet_balance(&self) -> f64 {
        self.max_drawdown_wallet_balance
    }

    /// Maximum drawdown of the wallet balance including unrealized profit and
    /// loss
    #[inline(always)]
    pub fn max_drawdown_total(&self) -> f64 {
        self.max_drawdown_total
    }

    /// Return the number of trades the account made
    #[inline(always)]
    pub fn num_trades(&self) -> i64 {
        self.num_trades
    }

    /// Return the ratio of executed trades vs total trading opportunities
    /// Higher values means a more active trading agent
    #[inline(always)]
    pub fn trade_percentage(&self) -> f64 {
        self.num_trades as f64 / self.num_trading_opportunities as f64
    }

    /// Return the ratio of buy trades vs total number of trades
    #[inline(always)]
    pub fn buy_ratio(&self) -> f64 {
        self.num_buys as f64 / self.num_trades as f64
    }

    /// Return the cumulative turnover denoted in margin currency
    #[inline(always)]
    pub fn turnover(&self) -> f64 {
        self.total_turnover
    }

    /// Return the total realized profit and loss of the account
    #[inline(always)]
    pub fn total_rpnl(&self) -> f64 {
        self.total_rpnl
    }

    /// Return the current unrealized profit and loss
    #[inline(always)]
    pub fn upnl(&self) -> f64 {
        self.upnl
    }

    /// Return the ratio of winning trades vs all trades
    #[inline]
    pub fn win_ratio(&self) -> f64 {
        if self.num_wins + self.num_losses > 0 {
            self.num_wins as f64 / (self.num_wins + self.num_losses) as f64
        } else {
            0.0
        }
    }

    /// Return the ratio of filled limit orders vs number of submitted limit
    /// orders
    #[inline(always)]
    pub fn limit_order_fill_ratio(&self) -> f64 {
        self.num_filled_limit_orders as f64 / self.num_submitted_limit_orders as f64
    }

    /// Return the ratio of limit order cancellations vs number of submitted
    /// limit orders
    #[inline(always)]
    pub fn limit_order_cancellation_ratio(&self) -> f64 {
        self.num_cancelled_limit_orders as f64 / self.num_submitted_limit_orders as f64
    }
}

impl AccountTracker for FullAccountTracker {
    fn update(&mut self, timestamp: u64, price: f64, upnl: f64) {
        self.price_last = price;
        if self.price_a_day_ago == 0.0 {
            self.price_a_day_ago = price;
        }
        if self.price_an_hour_ago == 0.0 {
            self.price_an_hour_ago = price;
        }
        if self.price_a_tick_ago == 0.0 {
            self.price_a_tick_ago = price;
        }
        if self.price_first == 0.0 {
            self.price_first = price;
        }
        self.num_trading_opportunities += 1;
        if self.ts_first == 0 {
            self.ts_first = timestamp;
        }
        self.ts_last = timestamp;
        if timestamp > self.next_daily_trigger_ts {
            self.next_daily_trigger_ts = timestamp + DAILY_NS;

            // calculate daily return of account
            let pnl: f64 = (self.total_rpnl + upnl) - self.last_daily_pnl;
            self.hist_returns_daily_acc.push(pnl);

            // calculate daily log return of account
            let ln_ret: f64 = ((self.wallet_balance_last + upnl)
                / (self.wallet_balance_start + self.last_daily_pnl))
                .ln();
            self.hist_ln_returns_daily_acc.push(ln_ret);

            // calculate daily return of buy_and_hold
            let bnh_qty = self.wallet_balance_start / self.price_first;
            let pnl_bnh = self.futures_type.pnl(self.price_a_day_ago, price, bnh_qty);
            self.hist_returns_daily_bnh.push(pnl_bnh);

            // calculate daily log return of market
            let ln_ret: f64 = (price / self.price_a_day_ago).ln();
            self.hist_ln_returns_daily_bnh.push(ln_ret);

            self.last_daily_pnl = self.total_rpnl + upnl;
            self.price_a_day_ago = price;
        }
        if timestamp > self.next_hourly_trigger_ts {
            self.next_hourly_trigger_ts = timestamp + HOURLY_NS;

            // calculate hourly return of account
            let pnl: f64 = (self.total_rpnl + upnl) - self.last_hourly_pnl;
            self.hist_returns_hourly_acc.push(pnl);

            // calculate hourly logarithmic return of account
            let ln_ret: f64 = ((self.wallet_balance_last + upnl)
                / (self.wallet_balance_start + self.last_hourly_pnl))
                .ln();
            self.hist_ln_returns_hourly_acc.push(ln_ret);

            // calculate hourly return of buy_and_hold
            let bnh_qty = self.wallet_balance_start / self.price_first;
            let pnl_bnh = self.futures_type.pnl(self.price_an_hour_ago, price, bnh_qty);
            self.hist_returns_hourly_bnh.push(pnl_bnh);

            // calculate hourly logarithmic return of buy_and_hold
            let ln_ret: f64 = (price / self.price_an_hour_ago).ln();
            self.hist_ln_returns_hourly_bnh.push(ln_ret);

            self.last_hourly_pnl = self.total_rpnl + upnl;
            self.price_an_hour_ago = price;
        }
        // compute tick-by-tick return statistics
        let pnl: f64 = (self.total_rpnl + upnl) - self.last_tick_pnl;
        self.hist_returns_tick_acc.push(pnl);

        let ln_ret: f64 = ((self.wallet_balance_last + upnl)
            / (self.wallet_balance_start + self.last_tick_pnl))
            .ln();
        self.hist_ln_returns_tick_acc.push(ln_ret);

        let bnh_qty = self.wallet_balance_start / self.price_first;
        let pnl_bnh: f64 = self.futures_type.pnl(self.price_a_tick_ago, price, bnh_qty);
        self.hist_returns_tick_bnh.push(pnl_bnh);

        let ln_ret = (price / self.price_a_tick_ago).ln();
        self.hist_ln_returns_tick_bnh.push(ln_ret);

        self.last_tick_pnl = self.total_rpnl + upnl;
        self.price_a_tick_ago = price;

        // update max_drawdown_total
        let curr_dd = (self.wallet_balance_high - (self.wallet_balance_last + upnl))
            / self.wallet_balance_high;
        if curr_dd > self.max_drawdown_total {
            self.max_drawdown_total = curr_dd;
        }
    }

    fn log_rpnl(&mut self, rpnl: f64) {
        self.total_rpnl += rpnl;
        self.wallet_balance_last += rpnl;
        if rpnl < 0.0 {
            self.total_loss += rpnl.abs();
            self.num_losses += 1;
        } else {
            self.num_wins += 1;
            self.total_profit += rpnl;
        }
        if self.wallet_balance_last > self.wallet_balance_high {
            self.wallet_balance_high = self.wallet_balance_last;
        }
        let dd = (self.wallet_balance_high - self.wallet_balance_last) / self.wallet_balance_high;
        if dd > self.max_drawdown_wallet_balance {
            self.max_drawdown_wallet_balance = dd;
        }
    }

    #[inline(always)]
    fn log_fee(&mut self, fee: f64) {
        self.cumulative_fees += fee
    }

    #[inline(always)]
    fn log_limit_order_submission(&mut self) {
        self.num_submitted_limit_orders += 1;
    }

    #[inline(always)]
    fn log_limit_order_cancellation(&mut self) {
        self.num_cancelled_limit_orders += 1;
    }

    #[inline(always)]
    fn log_limit_order_fill(&mut self) {
        self.num_filled_limit_orders += 1;
    }

    fn log_trade(&mut self, side: Side, price: f64, size: f64) {
        self.total_turnover += match self.futures_type {
            FuturesTypes::Linear => size * price,
            FuturesTypes::Inverse => size / price,
        };
        self.num_trades += 1;
        match side {
            Side::Buy => self.num_buys += 1,
            Side::Sell => {}
        }
    }
}

impl Display for FullAccountTracker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "
rpnl: {},
annualized_roi: {},
sharpe_daily_returns: {},
sharpe_hourly_returns: {},
sharpe_tick_returns: {},
sortino_daily_returns: {},
sortino_hourly_returns: {},
sortino_tick_returns: {},
drawdown_wallet_balance: {},
drawdown_total: {},
historical_value_at_risk_daily: {},
historical_value_at_risk_hourly: {},
cornish_fisher_value_at_risk_daily: {},
cornish_fisher_value_at_risk_daily_from_hourly_returns: {},
d_ratio_daily: {},
d_ratio_hourly: {},
d_ratio_tick: {},
num_trades: {},
buy_ratio: {},
turnover: {},
win_ratio: {},
profit_loss_ratio: {},
buy_and_hold_returns: {},
trade_percentage: {},
cumulative_fees: {},
num_trading_days: {},
            ",
            self.total_rpnl(),
            self.annualized_roi(),
            self.sharpe(ReturnsSource::Daily, true),
            self.sharpe(ReturnsSource::Hourly, true),
            self.sharpe(ReturnsSource::TickByTick, true),
            self.sortino(ReturnsSource::Daily, true),
            self.sortino(ReturnsSource::Hourly, true),
            self.sortino(ReturnsSource::TickByTick, true),
            self.max_drawdown_wallet_balance(),
            self.max_drawdown_total(),
            self.historical_value_at_risk(ReturnsSource::Daily, 0.01),
            self.historical_value_at_risk_from_n_hourly_returns(24, 0.01),
            self.cornish_fisher_value_at_risk(ReturnsSource::Daily, 0.01),
            self.cornish_fisher_value_at_risk_from_n_hourly_returns(24, 0.01),
            self.d_ratio(ReturnsSource::Daily),
            self.d_ratio(ReturnsSource::Hourly),
            self.d_ratio(ReturnsSource::TickByTick),
            self.num_trades(),
            self.buy_ratio(),
            self.turnover(),
            self.win_ratio(),
            self.profit_loss_ratio(),
            self.buy_and_hold_return(),
            self.trade_percentage(),
            self.cumulative_fees(),
            self.num_trading_days(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::round;

    // Some example hourly ln returns of BCHEUR i pulled from somewhere from about
    // october 2021
    const LN_RETS_H: [f64; 400] = [
        0.00081502,
        0.00333945,
        0.01293622,
        -0.00477679,
        -0.01195175,
        0.00750783,
        0.00426066,
        0.01214974,
        0.00892472,
        0.00344957,
        0.00684050,
        -0.00492310,
        0.00322274,
        0.02181239,
        0.00592118,
        0.00122343,
        -0.00623743,
        -0.00273835,
        0.01127133,
        -0.07646319,
        0.07090849,
        -0.00494601,
        -0.00624408,
        0.00256976,
        0.00130659,
        0.00098106,
        -0.00635020,
        0.00191424,
        -0.00306103,
        0.00640057,
        -0.00550237,
        0.00469525,
        0.00207676,
        -0.00449422,
        0.00472523,
        -0.00459109,
        -0.00382578,
        0.00420916,
        -0.01085029,
        0.00277287,
        -0.00929482,
        0.00680648,
        -0.00772934,
        -0.00250064,
        -0.01213199,
        -0.00098276,
        -0.00441975,
        0.00118162,
        0.00318254,
        -0.00314559,
        -0.00210387,
        0.00452694,
        -0.00116603,
        -0.00240180,
        0.00188400,
        0.00442843,
        -0.00769548,
        0.00154913,
        0.00447643,
        0.00081605,
        -0.00081605,
        -0.00201872,
        0.00183335,
        0.00540848,
        -0.01165400,
        0.00293312,
        0.00133104,
        -0.00555275,
        0.00309541,
        -0.01556380,
        -0.00101692,
        -0.00094336,
        -0.00039885,
        0.00121517,
        0.00312631,
        -0.00452272,
        -0.00484508,
        0.00718562,
        0.00252812,
        -0.00085555,
        0.00582124,
        0.00917446,
        -0.00847876,
        0.00492033,
        -0.00139778,
        -0.00511463,
        0.00474712,
        -0.00256881,
        0.00185255,
        -0.00276838,
        -0.00118933,
        0.01393963,
        0.00211617,
        -0.00733174,
        0.00223456,
        0.00331485,
        -0.00812862,
        0.00127036,
        0.01245729,
        -0.01264150,
        0.00075547,
        -0.00219115,
        0.00163830,
        -0.00734218,
        0.00730533,
        -0.00090229,
        -0.00585425,
        0.00370310,
        -0.00388606,
        0.00350045,
        -0.00593072,
        0.00756601,
        0.02024774,
        0.01012805,
        0.00128986,
        -0.00030365,
        -0.01334484,
        -0.00177715,
        -0.00373107,
        0.00792646,
        0.00013139,
        -0.00342925,
        0.01376916,
        0.00051222,
        0.00475530,
        -0.01058291,
        -0.00384123,
        -0.00663085,
        0.00141987,
        -0.00084096,
        -0.00953725,
        -0.00181163,
        -0.00127357,
        0.00040589,
        -0.00053500,
        0.00271486,
        -0.00024039,
        0.00613869,
        -0.00222986,
        -0.00340949,
        -0.00190351,
        0.00934898,
        0.00117479,
        -0.00102569,
        0.00003728,
        0.00257564,
        0.00893534,
        -0.00150733,
        -0.00645575,
        -0.00572640,
        0.00951222,
        -0.02857972,
        0.00519596,
        0.00908435,
        -0.00122096,
        -0.00510812,
        0.00103059,
        -0.00003682,
        -0.00266620,
        0.00473049,
        0.00377094,
        0.03262131,
        -0.00294230,
        -0.00281953,
        -0.00362701,
        -0.00001896,
        0.00212520,
        0.00367280,
        -0.00188566,
        0.00647177,
        -0.00816393,
        0.00705369,
        0.00903244,
        -0.00235244,
        0.01674118,
        -0.00652002,
        0.02306826,
        0.00615165,
        0.00122285,
        -0.00276431,
        0.00962792,
        0.01871500,
        -0.00793240,
        0.00881768,
        0.00592885,
        0.02721942,
        0.00850996,
        -0.01381862,
        0.00936217,
        -0.00407480,
        0.00236606,
        -0.00513002,
        0.01970497,
        -0.01412668,
        0.01755395,
        -0.00895548,
        0.00511687,
        0.00296984,
        0.02988059,
        -0.02572539,
        -0.00835808,
        0.00918683,
        0.00781964,
        0.00013195,
        -0.00880214,
        -0.01109966,
        -0.00734618,
        0.00665653,
        -0.01180100,
        0.00818809,
        0.00311751,
        -0.00260218,
        0.00804343,
        -0.00705497,
        0.01304860,
        0.02186613,
        -0.00044516,
        0.00443816,
        0.02123462,
        -0.00900067,
        0.02808619,
        -0.00069790,
        0.00723525,
        -0.03541517,
        0.00054277,
        0.00457999,
        0.00391639,
        -0.00836064,
        -0.00862783,
        -0.00347063,
        0.00661578,
        -0.00616864,
        -0.00129618,
        0.01089079,
        -0.00963933,
        -0.00265747,
        -0.00609216,
        -0.01428360,
        -0.00690326,
        0.00598589,
        -0.00141808,
        -0.00766637,
        -0.00563078,
        0.00103317,
        -0.00549794,
        -0.00339958,
        0.01535745,
        -0.00779424,
        -0.00051603,
        -0.00689776,
        0.00672581,
        0.00489062,
        -0.01046298,
        -0.00153764,
        0.01137449,
        0.00019427,
        0.00352505,
        0.01106645,
        -0.00325858,
        -0.01342477,
        0.00084053,
        0.00735775,
        -0.00149757,
        -0.01594285,
        0.00096097,
        -0.00549709,
        0.00603137,
        -0.00027786,
        -0.00243330,
        -0.00095889,
        0.00223883,
        0.00900579,
        0.00107754,
        0.00365070,
        0.00015150,
        0.00153795,
        0.00685195,
        -0.01102705,
        0.01336526,
        0.06330828,
        0.01472186,
        -0.00948722,
        0.00951088,
        -0.02122735,
        -0.00657814,
        0.00736579,
        -0.00494730,
        0.00945349,
        -0.00910751,
        0.00156993,
        -0.01752120,
        -0.00516317,
        -0.00036133,
        0.01299930,
        -0.00960670,
        -0.00695372,
        0.00358371,
        -0.00248066,
        -0.00085553,
        0.01013308,
        -0.01031310,
        0.01391146,
        -0.00500684,
        -0.01070302,
        0.00551785,
        0.01211034,
        -0.00066270,
        -0.00748760,
        0.01321500,
        -0.00914815,
        0.00367207,
        -0.00230517,
        0.00171125,
        -0.00573824,
        -0.00231329,
        0.00798303,
        -0.01103654,
        -0.00069986,
        0.01773706,
        0.00760968,
        -0.00032401,
        -0.00831888,
        0.00282665,
        0.00401237,
        0.00646741,
        0.02859090,
        0.00270779,
        -0.05185343,
        0.01053533,
        -0.00342470,
        -0.00574274,
        -0.00148180,
        -0.00443228,
        -0.00244637,
        0.01041581,
        0.00580057,
        -0.00174600,
        -0.00167422,
        -0.00006874,
        0.00696707,
        0.01696395,
        -0.00887856,
        -0.01404375,
        -0.00735852,
        0.00454126,
        0.00451603,
        -0.00009190,
        -0.00279887,
        0.00881306,
        0.00254559,
        -0.00333110,
        0.00718494,
        -0.00642254,
        -0.00157037,
        0.00406956,
        0.00896032,
        0.00668507,
        -0.00638110,
        0.00457055,
        -0.00124432,
        0.00211392,
        -0.00490214,
        0.00855329,
        -0.01061018,
        0.00374296,
        0.01959687,
        -0.00374546,
        -0.00886619,
        0.00798554,
        -0.00540965,
        -0.00297704,
        0.00608164,
        0.00523561,
        0.01267846,
        -0.00429216,
        -0.01136444,
        0.00498445,
        -0.01758464,
        0.01302850,
        -0.00007140,
        0.01033403,
        0.00269672,
        0.00674951,
        0.00206539,
        -0.00862200,
        0.00393849,
        -0.00504716,
        -0.00120369,
        0.01363795,
        0.00965599,
        -0.01106959,
        0.00534806,
        -0.01509123,
        -0.00450012,
        -0.00187109,
        0.00254361,
        -0.00813596,
        0.00054829,
        0.00250690,
        0.00753453,
    ];

    #[test]
    fn acc_tracker_profit_loss_ratio() {
        let mut at = FullAccountTracker::new(100.0, FuturesTypes::Linear);
        at.total_profit = 50.0;
        at.total_loss = 25.0;
        assert_eq!(at.profit_loss_ratio(), 2.0);
    }

    #[test]
    fn acc_tracker_cumulative_fees() {
        let mut at = FullAccountTracker::new(100.0, FuturesTypes::Linear);
        at.log_fee(0.1);
        at.log_fee(0.2);
        assert_eq!(round(at.cumulative_fees(), 1), 0.3);
    }

    #[test]
    fn acc_tracker_buy_and_hold_return() {
        let mut at = FullAccountTracker::new(100.0, FuturesTypes::Linear);
        at.update(0, 100.0, 0.0);
        at.update(0, 200.0, 0.0);
        assert_eq!(at.buy_and_hold_return(), 100.0);
    }

    #[test]
    fn acc_tracker_sell_and_hold_return() {
        let mut at = FullAccountTracker::new(100.0, FuturesTypes::Linear);
        at.update(0, 100.0, 0.0);
        at.update(0, 50.0, 0.0);
        assert_eq!(at.sell_and_hold_return(), 50.0);
    }

    #[test]
    fn acc_tracker_log_rpnl() {
        let rpnls: Vec<f64> = vec![0.1, -0.1, 0.1, 0.2, -0.1];
        let mut acc_tracker = FullAccountTracker::new(1.0, FuturesTypes::Linear);
        for r in rpnls {
            acc_tracker.log_rpnl(r);
        }

        assert_eq!(round(acc_tracker.max_drawdown_wallet_balance(), 2), 0.09);
        assert_eq!(round(acc_tracker.total_rpnl(), 1), 0.20);
    }

    #[test]
    fn acc_tracker_buy_and_hold() {
        let mut acc_tracker = FullAccountTracker::new(100.0, FuturesTypes::Linear);
        acc_tracker.update(0, 100.0, 0.0);
        acc_tracker.update(0, 200.0, 0.0);
        assert_eq!(acc_tracker.buy_and_hold_return(), 100.0);
    }

    #[test]
    fn acc_tracker_sell_and_hold() {
        let mut acc_tracker = FullAccountTracker::new(100.0, FuturesTypes::Linear);
        acc_tracker.update(0, 100.0, 0.0);
        acc_tracker.update(0, 200.0, 0.0);
        assert_eq!(acc_tracker.sell_and_hold_return(), -100.0);
    }

    #[test]
    fn acc_tracker_historical_value_at_risk() {
        if let Err(_e) = pretty_env_logger::try_init() {}

        let mut acc_tracker = FullAccountTracker::new(100.0, FuturesTypes::Linear);
        acc_tracker.hist_ln_returns_hourly_acc = LN_RETS_H.into();

        assert_eq!(
            round(acc_tracker.historical_value_at_risk(ReturnsSource::Hourly, 0.05), 3),
            1.173
        );
        assert_eq!(
            round(acc_tracker.historical_value_at_risk(ReturnsSource::Hourly, 0.01), 3),
            2.54
        );
    }

    #[test]
    fn acc_tracker_historical_value_at_risk_from_n_hourly_returns() {
        if let Err(_) = pretty_env_logger::try_init() {}

        let mut at = FullAccountTracker::new(100.0, FuturesTypes::Linear);
        at.hist_ln_returns_hourly_acc = LN_RETS_H.into();

        assert_eq!(round(at.historical_value_at_risk_from_n_hourly_returns(24, 0.05), 3), 3.835);
        assert_eq!(round(at.historical_value_at_risk_from_n_hourly_returns(24, 0.01), 3), 6.061);
    }

    #[test]
    fn acc_tracker_cornish_fisher_value_at_risk() {
        if let Err(_e) = pretty_env_logger::try_init() {}

        let mut acc_tracker = FullAccountTracker::new(100.0, FuturesTypes::Linear);
        acc_tracker.hist_ln_returns_hourly_acc = LN_RETS_H.into();

        assert_eq!(
            round(acc_tracker.cornish_fisher_value_at_risk(ReturnsSource::Hourly, 0.05), 3),
            1.354
        );
        assert_eq!(
            round(acc_tracker.cornish_fisher_value_at_risk(ReturnsSource::Hourly, 0.01), 3),
            5.786
        );
    }

    #[test]
    fn acc_tracker_cornish_fisher_value_at_risk_from_n_hourly_returns() {
        if let Err(_) = pretty_env_logger::try_init() {}

        let mut at = FullAccountTracker::new(100.0, FuturesTypes::Linear);
        at.hist_ln_returns_hourly_acc = LN_RETS_H.into();

        assert_eq!(
            round(at.cornish_fisher_value_at_risk_from_n_hourly_returns(24, 0.05), 3),
            4.043
        );
        assert_eq!(
            round(at.cornish_fisher_value_at_risk_from_n_hourly_returns(24, 0.01), 3),
            5.358
        );
    }
}