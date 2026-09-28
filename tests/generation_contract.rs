use serde_json::Value;
use std::fs;

const EXPECTED_AUTHORITY: &str = "7634e94c2051ec473a625ac13a83f329eb66ce2c";
const EXPECTED_LIFECYCLE: [&str; 8] = [
    "prepare",
    "validate",
    "compile_build_generation",
    "stage",
    "health_check",
    "atomic_activate",
    "bounded_drain",
    "commit",
];

#[test]
fn generation_contract_matches_current_shared_authority() {
    let raw = fs::read_to_string("ores-generation-contract.json")
        .expect("generation contract must be readable");
    let contract: Value = serde_json::from_str(&raw).expect("generation contract must be valid JSON");

    assert_eq!(
        contract["schema"].as_str(),
        Some("ores.desktop-generation-consumer/v1")
    );
    assert_eq!(
        contract["consumer"]["repository"].as_str(),
        Some("gha-indie-worker/giw-desktop-cli")
    );
    assert_eq!(contract["consumer"]["role"].as_str(), Some("desktop_cli"));
    assert_eq!(
        contract["authority"]["repository"].as_str(),
        Some("ORESoftware/ores-common-desktop-infra")
    );
    assert_eq!(
        contract["authority"]["revision"].as_str(),
        Some(EXPECTED_AUTHORITY)
    );

    let lifecycle = contract["lifecycle"]
        .as_array()
        .expect("lifecycle must be an array")
        .iter()
        .map(|value| value.as_str().expect("lifecycle entries must be strings"))
        .collect::<Vec<_>>();
    assert_eq!(lifecycle, EXPECTED_LIFECYCLE);

    assert_eq!(contract["rollback"]["required_before_commit"], true);
    assert_eq!(contract["rollback"]["retain_previous_generation"], true);
    assert_eq!(
        contract["request_semantics"]["new_requests"].as_str(),
        Some("active_generation")
    );
    assert_eq!(
        contract["request_semantics"]["existing_requests"].as_str(),
        Some("pinned_generation")
    );
    assert_eq!(
        contract["request_semantics"]["generation_identity_required"],
        true
    );
    assert_eq!(contract["routing"]["edge_proxy_route_authority"], false);
    assert_eq!(
        contract["middleware"]["beam_code_reload_requires_drain_or_otp_proof"],
        true
    );
    assert_eq!(
        contract["verification"]["shared_conformance_required"],
        true
    );
    assert_eq!(contract["verification"]["product_e2e_required"], true);
}
