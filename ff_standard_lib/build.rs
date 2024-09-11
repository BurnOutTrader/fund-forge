use std::{env, fs};
use std::fs::File;
use std::path::{Path, PathBuf};

    fn main() {
        eprintln!("Building with build.rs");

        // Set the PROTOC environment variable
        env::set_var("PROTOC", "/opt/homebrew/bin/protoc");

        // Set the path to the directory containing the proto files
        let proto_dir = PathBuf::from("src/apis/rithmic/proto/");
        let proto_file_path = proto_dir.join("account_list_updates.proto");
        env::set_var("OUT_DIR", "src/apis/rithmic/rithmic_proto_objects");
        // Check if proto directory and file exist
        if !proto_dir.exists() {
            panic!()
        }

        if !proto_file_path.exists() {
           panic!()
        }

        // Set the output directory within your Rust project
        let out_dir = PathBuf::from("src/apis/rithmic/rithmic_proto_objects");

        // Ensure the output directory exists
        if let Err(e) = fs::create_dir_all(&out_dir) {
            panic!()
        }

        let mut paths = vec![];

        for file in RITHMIC_PROTO_FILES {
            let proto_file_path = proto_dir.join(file);
            if proto_file_path.exists() {
                paths.push(proto_file_path.to_string_lossy().to_string());
            }
        }

        // Convert to Vec<&str> if needed
        let paths_as_str: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();

        // Compile the proto files
        match prost_build::compile_protos(&paths_as_str, &[proto_dir.to_str().unwrap()]) {
            Ok(_) => eprintln!("Protos compiled successfully"),
            Err(e) => panic!()
        }
    }

const RITHMIC_PROTO_FILES: [&str; 151] = [
    "account_list_updates.proto",
    "account_pnl_position_update.proto",
    "account_rms_updates.proto",
    "best_bid_offer.proto",
    "bracket_updates.proto",
    "depth_by_order_end_event.proto",
    "depth_by_order.proto",
    "end_of_day_prices.proto",
    "exchange_order_notification.proto",
    "forced_logout.proto",
    "front_month_contract_update.proto",
    "indicator_prices.proto",
    "instrument_pnl_position_update.proto",
    "last_trade.proto",
    "market_mode.proto",
    "message_type.proto",
    "open_interest.proto",
    "order_book.proto",
    "order_price_limits.proto",
    "otps_proto_pool.proto",
    "quote_statistics.proto",
    "reject.proto",
    "request_accept_agreement.proto",
    "request_account_list.proto",
    "request_account_rms_info.proto",
    "request_account_rms_updates.proto",
    "request_auxilliary_reference_data.proto",
    "request_bracket_order.proto",
    "request_cancel_all_orders.proto",
    "request_cancel_order.proto",
    "request_depth_by_order_snapshot.proto",
    "request_depth_by_order_updates.proto",
    "request_easy_to_borrow_list.proto",
    "request_exit_position.proto",
    "request_front_month_contract.proto",
    "request_get_instrument_by_underlying.proto",
    "request_get_volume_at_price.proto",
    "request_give_tick_size_type_table.proto",
    "request_heartbeat.proto",
    "request_link_orders.proto",
    "request_list_accepted_agreements.proto",
    "request_list_exchange_permissions.proto",
    "request_list_unaccepted_agreements.proto",
    "request_login_info.proto",
    "request_login.proto",
    "request_logout.proto",
    "request_market_data_update_by_underlying.proto",
    "request_market_data_update.proto",
    "request_modify_order_reference_data.proto",
    "request_modify_order.proto",
    "request_new_order.proto",
    "request_oco_order.proto",
    "request_order_session_config.proto",
    "request_pnl_position_snapshot.proto",
    "request_pnl_position_updates.proto",
    "request_product_codes.proto",
    "request_product_rms_info.proto",
    "request_reference_data.proto",
    "request_replay_executions.proto",
    "request_resume_bars.proto",
    "request_rithmic_system_gateway_info.proto",
    "request_rithmic_system_info.proto",
    "request_search_symbols.proto",
    "request_set_rithmic_mrkt_data_self_cert_status.proto",
    "request_show_agreement.proto",
    "request_show_bracket_stops.proto",
    "request_show_brackets.proto",
    "request_show_order_history_dates.proto",
    "request_show_order_history_detail.proto",
    "request_show_order_history_summary.proto",
    "request_show_order_history.proto",
    "request_show_orders.proto",
    "request_subscribe_for_order_updates.proto",
    "request_subscribe_to_bracket_updates.proto",
    "request_tick_bar_replay.proto",
    "request_tick_bar_update.proto",
    "request_time_bar_replay.proto",
    "request_time_bar_update.proto",
    "request_trade_routes.proto",
    "request_update_stop_bracket_level.proto",
    "request_update_target_bracket_level.proto",
    "request_volume_profile_minute_bars.proto",
    "response_accept_agreement.proto",
    "response_account_list.proto",
    "response_account_rms_info.proto",
    "response_account_rms_updates.proto",
    "response_auxilliary_reference_data.proto",
    "response_bracket_order.proto",
    "response_cancel_all_orders.proto",
    "response_cancel_order.proto",
    "response_depth_by_order_snapshot.proto",
    "response_depth_by_order_updates.proto",
    "response_easy_to_borrow_list.proto",
    "response_exit_position.proto",
    "response_front_month_contract.proto",
    "response_get_instrument_by_underlying_keys.proto",
    "response_get_instrument_by_underlying.proto",
    "response_get_volume_at_price.proto",
    "response_give_tick_size_type_table.proto",
    "response_heartbeat.proto",
    "response_link_orders.proto",
    "response_list_accepted_agreements.proto",
    "response_list_exchange_permissions.proto",
    "response_list_unaccepted_agreements.proto",
    "response_login_info.proto",
    "response_login.proto",
    "response_logout.proto",
    "response_market_data_update_by_underlying.proto",
    "response_market_data_update.proto",
    "response_modify_order_reference_data.proto",
    "response_modify_order.proto",
    "response_new_order.proto",
    "response_oco_order.proto",
    "response_order_session_config.proto",
    "response_pnl_position_snapshot.proto",
    "response_pnl_position_updates.proto",
    "response_product_codes.proto",
    "response_product_rms_info.proto",
    "response_reference_data.proto",
    "response_replay_executions.proto",
    "response_resume_bars.proto",
    "response_rithmic_system_gateway_info.proto",
    "response_rithmic_system_info.proto",
    "response_search_symbols.proto",
    "response_set_rithmic_mrkt_data_self_cert_status.proto",
    "response_show_agreement.proto",
    "response_show_bracket_stops.proto",
    "response_show_brackets.proto",
    "response_show_order_history_dates.proto",
    "response_show_order_history_detail.proto",
    "response_show_order_history_summary.proto",
    "response_show_order_history.proto",
    "response_show_orders.proto",
    "response_subscribe_for_order_updates.proto",
    "response_subscribe_to_bracket_updates.proto",
    "response_tick_bar_replay.proto",
    "response_tick_bar_update.proto",
    "response_time_bar_replay.proto",
    "response_time_bar_update.proto",
    "response_trade_routes.proto",
    "response_update_stop_bracket_level.proto",
    "response_update_target_bracket_level.proto",
    "response_volume_profile_minute_bars.proto",
    "rithmic_order_notification.proto",
    "symbol_margin_rate.proto",
    "tick_bar.proto",
    "time_bar.proto",
    "trade_route.proto",
    "trade_statistics.proto",
    "update_easy_to_borrow_list.proto",
    "user_account_update.proto",
];