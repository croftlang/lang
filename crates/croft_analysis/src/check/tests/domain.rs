use super::check_files;

const SHOP: &str = "context Shop {\n publish { Open }\n error Closed\n service Open(): Result<Unit, Closed> = Err(Closed)\n}";
const STOCK: &str =
    "context Stock {\n identity ItemId for Item: Int\n aggregate Item { id: ItemId }\n}";

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
fn reports_an_unknown_context() {
    let errors = check_files(&[SHOP, STOCK, &domain("Shop, Stock, Till", "Open()?")])
        .err()
        .unwrap();

    assert_eq!(errors, vec!["cannot find a context named `Till`"]);
}

const TILL: &str = "context Till {\n use { Open } from Shop\n}";

#[test]
fn lets_a_context_use_only_contexts_upstream_of_it() {
    assert!(check_files(&[SHOP, STOCK, TILL, &domain("Shop -> Till, Stock", "Open()?")]).is_ok());

    let errors = check_files(&[SHOP, STOCK, TILL, &domain("Shop, Stock, Till", "Open()?")])
        .err()
        .unwrap();

    assert_eq!(
        errors,
        vec!["`Till` uses `Shop`, so `contexts` needs `Shop -> Till`"]
    );
}

#[test]
fn reports_arrows_that_go_round_in_a_circle() {
    let errors = check_files(&[
        SHOP,
        STOCK,
        TILL,
        &domain("Shop -> Till -> Stock -> Shop", "Open()?"),
    ])
    .err()
    .unwrap();

    assert_eq!(
        errors,
        vec!["the arrows in `contexts` go round in a circle through `Shop`"]
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
