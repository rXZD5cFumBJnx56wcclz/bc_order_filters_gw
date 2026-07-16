use bc_order_filters::main_trait::*;
use bc_utils::other::procedure_used;
use bc_utils_lg::structs::settings::{SETTINGS_ORDER_FILTER, SETTINGS_ORDER_FILTERS};
use bc_utils_lg::structs::signals::Signal;
use bc_utils_lg::structs::trade::{Order, TradeState, Trigger};
use bc_utils_lg::types::maps::{FUNCS_EXTRACT_ARGS_TYPE, MAP};

pub fn get_map<'a>(
    s: &'a SETTINGS_ORDER_FILTERS,
    fa: &FUNCS_EXTRACT_ARGS_TYPE<SETTINGS_ORDER_FILTER, Box<dyn OrderFilter>>,
    orders: &[Option<&(Order, bool, Option<Trigger>)>],
    src: &[f64],
    signals: &[Signal],
    state: &TradeState,
) -> MAP<&'a str, (BF_ORDER_FILTER<'a>, Box<dyn OrderFilter>)> {
    s.iter()
        .map(|(k, setting)| {
            let order_filter = fa[setting.key.as_str()](setting);
            (
                k.as_str(),
                (order_filter.bf(orders, src, signals, state), order_filter),
            )
        })
        .collect()
}

pub fn get_orders<'a>(
    s: &SETTINGS_ORDER_FILTER,
    orders: &'a MAP<&str, (Order, bool, Option<Trigger>)>,
    orders_filtered: &MAP<&str, Option<&'a (Order, bool, Option<Trigger>)>>,
) -> Vec<Option<&'a (Order, bool, Option<Trigger>)>> {
    let mut res = Vec::with_capacity(s.used_orders.len() + s.used_orders_filtered.len());
    for used_orders in &s.used_orders {
        res.push(orders.get(used_orders.as_str()));
    }
    for used_orders_filtered in &s.used_orders_filtered {
        res.push(orders_filtered[used_orders_filtered.as_str()]);
    }
    res
}

pub fn get_src<'a>(
    s: &SETTINGS_ORDER_FILTER,
    buffer: &[Vec<f64>],
    indications: &'a MAP<&'a str, f64>,
    res_utils_state: &'a MAP<&'a str, f64>,
) -> Vec<f64> {
    let mut res =
        Vec::with_capacity(s.used_ind.len() + s.used_src.len() + s.used_utils_state.len());
    for used_src in &s.used_src {
        res.push(buffer[buffer.len() - used_src.sub_from_last_i][used_src.index]);
    }
    for used_ind in &s.used_ind {
        res.push(indications[used_ind.as_str()]);
    }
    for used_utils_state in &s.used_utils_state {
        res.push(res_utils_state[used_utils_state.as_str()]);
    }
    if !s.procedure_used_src.is_empty() {
        res = procedure_used(res, &s.procedure_used_src);
    }
    res
}

pub struct OrderFilterGateway<'a> {
    pub order_filters: *const MAP<&'a str, (BF_ORDER_FILTER<'a>, Box<dyn OrderFilter>)>,
    s: &'a SETTINGS_ORDER_FILTERS,
}

impl<'a> OrderFilterGateway<'a> {
    pub fn new(
        order_filters: *const MAP<&'a str, (BF_ORDER_FILTER<'a>, Box<dyn OrderFilter>)>,
        s: &'a SETTINGS_ORDER_FILTERS,
    ) -> Self {
        Self { order_filters, s }
    }
}

impl OrderFilterGateway<'_> {
    pub fn series<'a>(
        &'a self,
        orders: &'a MAP<&str, (Order, bool, Option<Trigger>)>,
        buffer: &[Vec<f64>],
        indications: &'a MAP<&'a str, f64>,
        res_utils_state: &'a MAP<&'a str, f64>,
        signals: &'a MAP<&'a str, Signal>,
        state: &TradeState,
    ) -> MAP<&'a str, Option<&'a (Order, bool, Option<Trigger>)>> {
        self.s
            .iter()
            .fold(MAP::default(), move |mut init, (k, setting)| {
                init.insert(k.as_str(), {
                    let (bf, order_filter) = &unsafe { &*self.order_filters }[k.as_str()];
                    order_filter.filter(
                        bf,
                        &get_orders(setting, orders, &init),
                        &get_src(setting, buffer, indications, res_utils_state),
                        &setting
                            .used_signals
                            .iter()
                            .map(|k_signal| signals[k_signal.as_str()])
                            .collect::<Vec<Signal>>(),
                        state,
                    )
                });
                init
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{default, sync::LazyLock};

    use bc_pack_order_filters::FUNCS_EXTRACT_ARGS as FA;
    use pretty_assertions::assert_eq as assert_eq_pr;

    static S: LazyLock<SETTINGS_ORDER_FILTERS> = LazyLock::new(|| {
        SETTINGS_ORDER_FILTERS::from_iter([
            (
                "count_1".to_string(),
                SETTINGS_ORDER_FILTER {
                    key: "count".to_string(),
                    kwargs_f64: MAP::from_iter([("max_count".to_string(), 2.)]),
                    used_orders: vec!["order_1".to_string()],
                    ..Default::default()
                },
            ),
            (
                "side_1".to_string(),
                SETTINGS_ORDER_FILTER {
                    key: "side".to_string(),
                    kwargs_string: MAP::from_iter([("side".to_string(), "buy".to_string())]),
                    used_orders: vec!["order_1".to_string()],
                    ..Default::default()
                },
            ),
        ])
    });
    static M: LazyLock<
        fn() -> MAP<&'static str, (BF_ORDER_FILTER<'static>, Box<dyn OrderFilter>)>,
    > = LazyLock::new(|| || get_map(&S, &FA, &[], &[], &[], &Default::default()));

    #[test]
    fn series_res_1() {
        assert_eq_pr!(
            OrderFilterGateway::new(&M(), &S).series(
                &MAP::from_iter([(
                    "order_1",
                    (
                        Order { side: "buy".to_string(), ..Default::default() },
                        Default::default(),
                        Default::default()
                    )
                ),]),
                &[],
                &Default::default(),
                &Default::default(),
                &Default::default(),
                &Default::default()
            ),
            MAP::from_iter([
                (
                    "count_1",
                    Some(&(
                        Order { side: "buy".to_string(), ..Default::default() },
                        Default::default(),
                        Default::default()
                    ))
                ),
                (
                    "side_1",
                    Some(&(
                        Order { side: "buy".to_string(), ..Default::default() },
                        Default::default(),
                        Default::default()
                    ))
                )
            ])
        );
    }
}
