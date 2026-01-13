**Order Types, Entry &amp; Functionalities**

**Current order types used:**

- **Pre-order**
    - Ideally, a pre-order in our system should be a finalized PO that is entered, or received via EDI, under the shipping DC at the time of sell-in season order deadlines. These orders generally don’t change, and are seen as committed buys with guaranteed inventory. We rely on and report against this revenue.

- **Re-order**
    - Re-orders should be used for orders beyond what the account has committed to at time of sell in, and can also “eat” from Bulks, or if no inventory available in bulk, would be considered reliant on “ATS” (available to sell) inventory. This order type should be indicative of a “chase” opportunity for an account, what went above and beyond the pre-seasonal plan due to high sell-through and business.

- **Bulk**
    - Bulks are generally used to replace what we previously called forecasts (in our previous order system, Salesforce), and are there to ensure production of and reserve additional stock the KAM believes will be necessary to support the in-season business of their accounts, aka “chase”. The incoming re-orders will search through available bulks on the parent accounts, and automatically utilize and balance the bulk inventory.

- Propose to rename this kind of Bulk in the system a “ **Forecast Bulk** ” to appropriately differentiate chase bulks vs. parent bulks.

**Challenges to note:**

- Our largest account, Harbor &amp; March, gives us PO’s (pre-orders) ahead of season and within deadline, but these orders are not “ship ready”, as they will later be broken out and allocated across their 7 DC’s and up to 200 stores (200 SO’s) prior to us releasing and shipping the order.

- These are committed buys from the account, and should be considered PRE-ORDERS. Functionally, they are a prime candidate to utilize the BULK SALES ORDER option, so orders reduce from the parent PO as the ship-ready orders come in via EDI 30-45 days ahead of ship dates, rather than us cancelling the “parent PO” placeholder when the allocated orders come in (this is a manual intervention that often gets missed, inflating the order book significantly and skewing reporting in multiple ways).

- If we cancel the parent pre-order we’ve placed under the parent account ahead of season, we also skew the pre-order cancellation reporting.

- If we utilize Bulks without further classification of the orders, we lose visibility and reporting into these being actual pre-orders, and not forecasted amounts to account for re-order potential (as bulks are intended to be used).

- We are currently unable to change the order type manually in the new ERP system. Accounts often mistakenly send PO’s via EDI with the wrong order type, or the EDI mapping is incorrect. This skews reporting of re-order business vs. pre-order business, also affecting allocation prioritization and incorrect bulk order reduction. Question: Should orders come in as drafts and only be confirmed once reviewed? Note, manual intervention reliant on human review. Can we adjust system to allow changes?

Goal based on above challenges:

- With a mutual goal to not to overcomplicate the business and systems, we can simplify understanding the Key Account business as a whole by adding a couple of order types into the system to clarify reporting.

- The goal here is to manage the KA order books better, so our KAM’s and functional internal partners can look at the books and have a clear understanding of the business, and are able to utilize that data to make relevant business decisions in the future. We should all ideally know exactly what is what, without investigating or inquiring into a PO.

- The KAM should be able to easily look at a previous season or year, and know what the account bought into vs. what they “chased”, and utilize the order to best advise the account on next season’s buy or clearly review the special opportunities such as Direct Shipments, Stock Clearance orders, etc.

- Demand planning should be able to look at the order books and clearly be able to differentiate which orders were committed buys, which were chase asks, which were forecasted for, and orders that were not accommodated through US-Stock (direct shipments).

- The logistics team should feel confident in the order book quantities so they can appropriately plan shipping resources by month. They should not have to question what is accurate or true, and should not be seeing inflated order books.

- Sales Ops and Customer Service teams should be able to understand which orders are committed buys and more highly prioritized for launch etc., while KA is a prioritized market, there is still some reliance on understanding whether an order is a “pre-order” or a “re-order”