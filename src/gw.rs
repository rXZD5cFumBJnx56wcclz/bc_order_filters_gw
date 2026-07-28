use bc_order_filters::main_trait::*;
use bc_utils::other::procedure_used;
use bc_utils_lg::structs::settings::{SETTINGS_ORDER_FILTER, SETTINGS_ORDER_FILTERS};
use bc_utils_lg::structs::signals::Signal;
use bc_utils_lg::structs::trade::{Order, TradeState, Trigger};
use bc_utils_lg::types::maps::{MAP, PACK};

pub fn get_orders<'a>(
    s: &SETTINGS_ORDER_FILTER,
    orders: &'a MAP<&'a str, (Order, bool, Option<Trigger>)>,
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

pub fn get_src_series<'a>(
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

pub type OrderFilters<'a> = MAP<&'a str, Box<dyn OrderFilter>>;

pub trait OrderFiltersExt<'a> {
    fn new(
        s: &'a SETTINGS_ORDER_FILTERS,
        fa: &PACK<SETTINGS_ORDER_FILTER, Box<dyn OrderFilter>>,
    ) -> Self;
}

pub trait OrderFilterUpdateBf {
    fn update_bf(&mut self);
}

impl<'a> OrderFiltersExt<'a> for OrderFilters<'a> {
    fn new(
        s: &'a SETTINGS_ORDER_FILTERS,
        fa: &PACK<SETTINGS_ORDER_FILTER, Box<dyn OrderFilter>>,
    ) -> Self {
        s.iter()
            .map(|(k, setting)| {
                let order_filter = fa[setting.key.as_str()](setting);
                (k.as_str(), order_filter)
            })
            .collect()
    }
}

impl<'a> OrderFilterUpdateBf for OrderFilters<'a> {
    fn update_bf(&mut self) {
        for f in self.values() {
            f.init_bf();
        }
    }
}

#[derive(Default)]
pub struct OrderFilterGateway<'a> {
    pub order_filters: *const OrderFilters<'a>,
    s: *const SETTINGS_ORDER_FILTERS,
}

impl<'a> OrderFilterGateway<'a> {
    pub fn new(
        order_filters: *const MAP<&'a str, Box<dyn OrderFilter>>,
        s: &'a SETTINGS_ORDER_FILTERS,
    ) -> Self {
        Self { order_filters, s }
    }
}

impl<'settings, 'order_link> OrderFilterGateway<'settings> {
    pub fn series(
        &self,
        orders: &'order_link MAP<&str, (Order, bool, Option<Trigger>)>,
        buffer: &[Vec<f64>],
        indications: &MAP<&str, f64>,
        res_utils_state: &MAP<&str, f64>,
        signals: &MAP<&str, Signal>,
        state: &TradeState,
    ) -> MAP<&'settings str, Option<&'order_link (Order, bool, Option<Trigger>)>> {
        unsafe { &*self.s }
            .iter()
            .fold(MAP::default(), move |mut init, (k, setting)| {
                init.insert(k.as_str(), {
                    let order_filter= &unsafe { &*self.order_filters }[k.as_str()];
                    order_filter.filter(
                        &get_orders(setting, orders, &init),
                        &get_src_series(setting, buffer, indications, res_utils_state),
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

    use std::sync::LazyLock;

    use bc_packs::PACK_ORDER_FILT;
    use pretty_assertions::assert_eq as assert_eq_pr;

    static S: LazyLock<SETTINGS_ORDER_FILTERS> = LazyLock::new(|| {
        SETTINGS_ORDER_FILTERS::from_iter([
            (
                "count_1".to_string(),
                SETTINGS_ORDER_FILTER {
                    key: "count".to_string(),
                    kwargs_usize: MAP::from_iter([("max_count".to_string(), 2)]),
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
        fn() -> MAP<&'static str, Box<dyn OrderFilter>>,
    > = LazyLock::new(|| || OrderFilters::new(&S, &PACK_ORDER_FILT));

    #[test]
    fn series_res_1() {
        assert_eq_pr!(
            OrderFilterGateway::new(&M(), &S).series(
                &MAP::from_iter([(
                    "order_1",
                    (
                        Order {
                            side: "buy".to_string(),
                            ..Default::default()
                        },
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
                        Order {
                            side: "buy".to_string(),
                            ..Default::default()
                        },
                        Default::default(),
                        Default::default()
                    ))
                ),
                (
                    "side_1",
                    Some(&(
                        Order {
                            side: "buy".to_string(),
                            ..Default::default()
                        },
                        Default::default(),
                        Default::default()
                    ))
                )
            ])
        );
    }
}
