// A small invoicing domain, used to try out sp end to end.

const MAX_LINES: Int = 100
const MAX_DESCRIPTION_LENGTH: Int = 200

enum Currency {
    Gbp
    Usd
    Eur
}

enum InvoiceStatus {
    Draft
    Issued
    Paid
}

enum SettleError {
    NotFound
    AlreadyPaid
    PaymentFailed
    StorageFailed
}

enum StorageError {
    Unavailable
}

enum PaymentError {
    Declined
    Unavailable
}

// ---- constraints ----

constraint Positive(amount: Int) = amount > 0
constraint NotBlank(text: String) = text.length > 0
constraint MaxLength(text: String, limit: Int) = text.length <= limit
constraint NonEmpty<T>(list: List<T>) = list.length > 0
constraint MaxItems<T>(list: List<T>, limit: Int) = list.length <= limit
constraint DueAfterIssue(invoice: Invoice) = invoice.due_day >= invoice.issue_day

// ---- structs ----

struct Money {
    // Amount in minor units, e.g. pence.
    amount: Int is Positive
    currency: Currency
}

struct InvoiceId {
    value: String is NotBlank
}

struct Line {
    description: String is NotBlank, MaxLength(MAX_DESCRIPTION_LENGTH)
    price: Money
}

struct Invoice {
    id: InvoiceId
    lines: List<Line> is NonEmpty, MaxItems(MAX_LINES)
    status: InvoiceStatus
    issue_day: Int
    due_day: Int
} is DueAfterIssue

struct ChargeRequest {
    invoice_id: InvoiceId
    total: Money
}

struct Receipt {
    invoice_id: InvoiceId
    total: Money
}

// ---- ports and adapters ----

port InvoiceRepository {
    fn find(id: InvoiceId) -> Option<Invoice>
    fn save(invoice: Invoice) -> Result<Unit, StorageError>
}

port PaymentGateway {
    fn charge(request: ChargeRequest) -> Result<Receipt, PaymentError>
}

adapter InMemoryInvoices for InvoiceRepository
adapter FakePayments for PaymentGateway

// ---- functions ----

fn scale_by_ten(value: Int) -> Int {
    requires value <= 1_000_000 && value >= -1_000_000
    ensures output == value * 10
    description "Multiplies a value by ten."
}

fn invoice_total(invoice: Invoice) -> Money {
    requires NonEmpty(invoice.lines)
    ensures Positive(output.amount)
    description "
        Adds up the price of every line on the invoice.
        All lines share one currency, which the total uses.
    "
}

fn settle_invoice(id: InvoiceId) -> Result<Receipt, SettleError> {
    uses InvoiceRepository, PaymentGateway
    requires NotBlank(id.value)
    description "
        Loads the invoice, charges its total through the payment gateway
        and saves it with status Paid. Fails with AlreadyPaid if the
        invoice has been paid before.
    "
}
