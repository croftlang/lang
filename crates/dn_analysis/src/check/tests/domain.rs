use super::check_files;

const SHOP: &str = "context Shop {\n publish { Open }\n error Closed\n service Open(): Result<Unit, Closed> = Err(Closed)\n}";
const STOCK: &str = "context Stock {\n identity ItemId: Int\n}";

fn domain(contexts: &str, body: &str) -> String {
    format!("domain Market {{\n contexts {{ {contexts} }}\n use {{ Open }} from Shop\n}} = {{\n{body}\n}}")
}

#[test]
fn accepts_a_domain_that_lists_every_context() {
    let market = domain("Shop, Stock", "Open()?");
    assert!(check_files(&[SHOP, STOCK, &market]).is_ok());
}

#[test]
fn reports_a_context_left_out_of_the_domain() {
    let errors = check_files(&[SHOP, STOCK, &domain("Shop", "Open()?")])
        .err()
        .unwrap();
    assert_eq!(
        errors,
        vec!["context `Stock` is not in the `contexts` of domain `Market`"]
    );
}

#[test]
fn reports_an_unknown_or_repeated_context() {
    let errors = check_files(&[SHOP, STOCK, &domain("Shop, Stock, Stock, Till", "Open()?")])
        .err()
        .unwrap();
    assert_eq!(
        errors,
        vec![
            "context `Stock` is listed more than once",
            "cannot find a context named `Till`"
        ]
    );
}

#[test]
fn reports_a_second_domain() {
    let second = "domain Bazaar {\n contexts { Shop, Stock }\n} = Unit";
    let errors = check_files(&[SHOP, STOCK, &domain("Shop, Stock", "Open()?"), second])
        .err()
        .unwrap();
    assert_eq!(
        errors,
        vec!["a program has one domain, and `Market` is already declared"]
    );
}

#[test]
fn lets_the_domain_pass_up_any_error_but_not_reach_inside_contexts() {
    assert!(check_files(&[SHOP, STOCK, &domain("Shop, Stock", "Open()?\n Open()?")]).is_ok());
    let market = "domain Market {\n contexts { Shop, Stock }\n use { Closed } from Shop\n} = Unit";
    let errors = check_files(&[SHOP, STOCK, market]).err().unwrap();
    assert_eq!(errors, vec!["context `Shop` does not publish `Closed`"]);
}
