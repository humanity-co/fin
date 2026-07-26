# COMPLIANCE DISCOVERY REPORT — Finance Module for SutraERP

**Prepared by:** CA (agent-ca)
**Date:** 2026-07-21
**Session:** 0c0cd34d-22dd-4ad7-8d8d-b2e6bc697bd3

---

## 1. GST FOR EDUCATIONAL INSTITUTIONS

### 1.1 Applicability — Exempt vs. Taxable

**Law reference:** Notification No. 12/2017-Central Tax (Rate) as amended; Entry 66 & 67

**Exempt (No GST):** Education services by a recognized institution to its students — tuition, admission, examination, library, lab fees.

**Taxable (GST applies):**

| Service | GST Rate | HSN/SAC |
|---------|----------|---------|
| Tuition/education fees | Exempt | 9992 |
| Hostel (≤₹1,000/day) | Exempt | 9963 |
| Hostel (>₹1,000/day) | 5% (no ITC) | 9963 |
| Mess/canteen (institution-run) | 5% (no ITC) | 9963 |
| Transportation | 5% | 9964 |
| Books | Exempt | 4901 |
| Stationery | 12% | 4905/4906 |
| Consultancy services | 18% | 9986 |
| Research grants (with service obligation) | 18% | 9986 |
| Workshop/STTP fees | 18% | 9992 |
| Rental of premises to third parties | 18% | 9972 |

### 1.2 Input Tax Credit (ITC) Rules

**Law reference:** Section 16 & 17(5) of CGST Act

- ITC blocked on inputs for exempt education services
- ITC reversal required under Rule 42/43 based on exempt-to-taxable turnover ratio
- ITC available on inputs used for taxable supplies (consultancy, workshop, hostel above threshold, canteen, transport, rental)

**ERP requirement:** Track ITC at invoice level; auto-calculate reversal under Rule 42/43.

### 1.3 Filing Requirements

| Return | Frequency | Due Date |
|--------|-----------|----------|
| GSTR-1 | Monthly/Quarterly (QRMP) | 11th/13th |
| GSTR-3B | Monthly | 20th |
| GSTR-9 (annual) | Annual | 31 Dec next FY (>₹2 Cr) |
| GSTR-9C (audit) | Annual | 31 Dec next FY (>₹5 Cr) |

### 1.4 Reverse Charge Mechanism (RCM) Scenarios

- Goods Transport Agency (GTA): 5%/12%
- Advocate/legal services: 18%
- Director/independent director services: 18%
- Sponsorship services: 18%
- Import of services: 18%
- Security services (unregistered provider)

**ERP requirement:** RCM flag at PO/invoice entry; auto-generate RCM payable entries.

---

## 2. TDS (TAX DEDUCTED AT SOURCE)

### 2.1 Key Sections

| Section | Nature | Rate | Threshold |
|---------|--------|------|-----------|
| 192 | Salaries | Per slab | Basic exemption limit |
| 194C | Contractor (individual/HUF) | 1% | ₹30,000 per contract / ₹1,00,000 aggregate |
| 194C | Contractor (other) | 2% | ₹30,000 per contract / ₹1,00,000 aggregate |
| 194J | Professional fees | 10% | ₹30,000 |
| 194I | Rent — plant/machinery | 2% | ₹2,40,000 |
| 194I | Rent — land/building | 10% | ₹2,40,000 |
| 194A | Interest | 10% | ₹40,000 |
| 194H | Commission | 5% | ₹15,000 |
| 194Q | Purchase of goods | 0.1% | ₹50 Lakhs |

### 2.2 Return Filing

| Form | Content | Due Date |
|------|---------|----------|
| 24Q | Salary TDS | 15th of month after quarter |
| 26Q | Non-salary TDS | 15th of month after quarter |
| 27Q | Non-resident TDS | 15th of month after quarter |
| Form 16 | Salary certificate | 31st May |
| Form 16A | Non-salary certificate | 15 days after return filing |

### 2.3 Lower/Nil Deduction Certificate (Section 197)

**ERP requirement:** Master of Section 197 certificates with vendor mapping, certificate number, validity period, specified rate. Auto-apply lower rate during payment processing. Expiry alerts.

---

## 3. MAHARASHTRA STATE SCHOLARSHIPS

### 3.1 Major Schemes

| Scheme | Beneficiary | Amount |
|--------|-------------|--------|
| Rajarshi Chhatrapati Shahu Maharaj Shikshan Shulkh Shishyavrutti | SC/ST (post-matric) | Up to 100% tuition + maintenance |
| EBC Scholarship | EBC students | Up to ₹50,000/year |
| PMS for SC | SC students | Full tuition + maintenance |
| PMS for ST | ST students | Full tuition + maintenance |
| PMS for OBC | OBC students | Tuition + maintenance |
| PMS for VJNT | VJNT/SBC students | Tuition + maintenance |

### 3.2 Disbursement Process (MahaDBT)

1. Student applies on MahaDBT portal (Aadhaar, bank, caste, income, fee receipt)
2. Institute verifies on MahaDBT (enrollment, attendance, fee structure)
3. Department sanctions
4. Direct Benefit Transfer (DBT) via PFMS to student's Aadhaar-linked account
5. Institute reconciles DBT against fee payable

### 3.3 ERP Requirements

- Student profile with caste, income, scholarship category
- Fee structure showing: Gross fee → Scholarship (expected/received) → Net payable → Concession
- Disbursement tracking (DBT date + amount)
- Refund handling if scholarship arrives after fee payment
- Scheme-wise reconciliation reports
- Scholarship audit trail (timestamped, user-attributed)
- Documentation: scholarship register, fee receipts, bank statements, Aadhaar verification

---

## 4. NAAC FINANCIAL REPORTING

### 4.1 Key Financial Metrics

| Metric | Data Required |
|--------|---------------|
| 3.3.1 | Grants received for research (year-wise, last 5 years) |
| 3.3.2 | Research grants per faculty |
| 3.3.3 | Revenue from consultancy |
| 3.4.4 | Budget allocation for research (% of total) |
| 5.1.1 | Scholarship/freeship expenditure |
| 7.1.1-4 | Environmental, gender, social initiative expenditure |

### 4.2 ERP Requirement

NAAC Dashboard with real-time 5-year financial metrics, auto-generated NAAC-format reports, research grant-to-faculty mapping, scholarship vs. budget tracking.

---

## 5. AISHE REPORTING

### 5.1 Financial Data Fields (Part C)

- Total Receipts: Government grants (recurring/non-recurring), other grants, tuition fees, other fees, examination fees, other receipts
- Total Expenditure: Teaching salaries, non-teaching salaries, maintenance, research, other
- Frequency: Annual, data as of 30th September
- Portal: aishe.gov.in

### 5.2 ERP Requirement

Map internal COA heads to AISHE reporting heads. Generate AISHE-compatible data extracts.

---

## 6. UGC FINANCIAL COMPLIANCE

### 6.1 Key Requirements

- Grants maintained in separate bank account/ledger
- Utilization Certificate (UC) in GFR 12-A format (audited, signed by Head + Statutory Auditor)
- Interest on grants must be reported
- Unspent balance tracking

### 6.2 Common UGC Grant Schemes

Development grants, Research projects (Major/Minor), Special grants (SAP, DRS, DSA), Salary grants (2f/12B), E-Governance, Equal Opportunity Cell

### 6.3 Endowment Fund (Maharashtra)

Maharashtra Self-Financed Universities Act, 2013: Corpus fund ₹10-20 Cr. Principal untouchable, income for development only.

### 6.4 ERP Requirement

Grant management module with fund-wise ledger, utilization tracking against approved budget heads, auto-generated UC in GFR 12-A, interest computation, unspent balance reports.

---

## 7. INCOME TAX FOR EDUCATIONAL TRUSTS/SOCIETIES

### 7.1 Exemptions

| Section | Applicability | Requirements |
|---------|---------------|--------------|
| 10(23C)(iiiad) | Receipts ≤₹1 Cr | Auto-exempt |
| 10(23C)(iiiab) | Educational institution, not for profit | Commissioner approval |
| 10(23C)(vi) | University/educational institution, receipts >₹1 Cr | CCIT approval |
| 11 & 12 | Trusts under 12A/12AB | 85% application rule |
| 12AB | Provisional registration | 3-year validity, renewal required |

### 7.2 Key Conditions

- 85% of income must be applied to educational purposes
- 15% can be accumulated (up to 5 years, for specified purposes)
- Funds must be invested in Section 11(5) specified securities
- No private benefit to trustees/founders/relatives

### 7.3 FCRA

- Mandatory for receiving foreign contributions
- Separate SBI New Delhi Main Branch account
- Administrative expenses ≤20% of FCRA receipts
- Annual FC-4 return by 31st December
- No re-granting of FCRA funds

### 7.4 Audit Requirements

| Type | Section | Deadline |
|------|---------|----------|
| Tax Audit | 44AB | 30 Sep |
| Trust Audit | 12A/12AB | 30 Sep |
| Form 10B | Income >₹5 Cr | 30 Sep |
| Form 10BB | Income ≤₹5 Cr | 30 Sep |
| ITR-7 | All trusts | 30 Sep |

---

## 8. MAHARASHTRA STATE-SPECIFIC

### 8.1 Professional Tax

| Monthly Salary | PT/Month |
|---------------|----------|
| ≤ ₹7,500 | Nil |
| ₹7,501 – ₹10,000 | ₹175 |
| ₹10,001+ | ₹200 |

- Registration: PT-EC within 30 days of becoming liable
- Returns: PT-1 (monthly) or PT-1A (half-yearly), PT-2 (annual by 31 May)

### 8.2 Fee Regulation (FRC)

- Fee structure approved by Maharashtra Fee Regulatory Committee
- Fee refund policy as per FRC guidelines
- Excess fee must be refunded/adjusted

### 8.3 Labour Welfare Fund

- 5+ employees
- Employee: ₹12/month, Employer: ₹24/month
- Payment: Half-yearly (June and December)

---

## 9. CROSS-CUTTING ERP REQUIREMENTS

1. **Compliant COA** — Default template for Maharashtra educational institutions, mappable to AISHE/NAAC/UGC heads
2. **Compliance calendar** — Built-in alerts for all filing deadlines
3. **Multi-GSTIN support** — Consolidated and per-campus returns
4. **Grant management** — Fund accounting with grant-wise ledger and budget control
5. **Scholarship integration** — MahaDBT/DBT workflow support
6. **Full audit trail** — Every transaction: who, when, approvals, change history
7. **Role-based access** — CFO, controller, accountant, registrar, auditor levels
8. **Auto-classification** — Income classified as exempt/taxable based on fee/service nature
9. **ITC tracking** — Invoice-level, auto-reversal under Rule 42/43
10. **RCM handling** — Flag at PO/invoice entry, auto-generate payable entries
