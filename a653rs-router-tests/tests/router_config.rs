use a653rs_router::prelude::*;
use a653rs_router_tests::test_data;

#[test]
fn main() {
    let cfg = test_data::CFG;
    let cfg: RouterConfig<8, 8, 8, 8> = serde_yaml::from_str(cfg).unwrap();
    println!("{cfg:?}");
}
