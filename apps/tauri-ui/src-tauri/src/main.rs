#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            edge_policy_tauri_ui::list_tenants,
            edge_policy_tauri_ui::get_tenant,
            edge_policy_tauri_ui::create_tenant,
            edge_policy_tauri_ui::update_tenant,
            edge_policy_tauri_ui::delete_tenant,
            edge_policy_tauri_ui::set_quota_limits,
            edge_policy_tauri_ui::compile_policy_dsl,
            edge_policy_tauri_ui::test_policy,
            edge_policy_tauri_ui::deploy_policy,
            edge_policy_tauri_ui::list_policy_bundles,
            edge_policy_tauri_ui::get_policy_bundle,
            edge_policy_tauri_ui::activate_policy_bundle,
            edge_policy_tauri_ui::rollback_policy,
            edge_policy_tauri_ui::query_audit_logs,
            edge_policy_tauri_ui::get_quota_metrics,
            edge_policy_tauri_ui::list_all_quota_metrics,
            edge_policy_tauri_ui::get_enforcer_ws_url,
            edge_policy_tauri_ui::check_quota_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running Edge Policy Hub application");
}
