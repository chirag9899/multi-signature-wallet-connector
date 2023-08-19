use cw_storage_plus::{Item, Map};
use crate::msg::DeployMsg;

pub const DEPLOY_DATA: Item<DeployMsg> = Item::new("deploy_data");
pub const GROUP_ADDR: Item<String> = Item::new("cw4_group_addr");
pub const USER_WALLETS: Map<String, Vec<String>> = Map::new("user_wallets");