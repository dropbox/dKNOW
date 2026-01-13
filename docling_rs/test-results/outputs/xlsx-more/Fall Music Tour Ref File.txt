| Income & Costs Reported by Tour Manager   |
|-------------------------------------------|
| 2024 Fall Music Tour                      |

| Description   |
|---------------|

| USD   |
|-------|

| INCOME              |
|---------------------|
| Tour Dates          |
| 2024-10-07 00:00:00 |
| =B9+2               |
| =B10+1              |
| =B11+2              |
| =B12+2              |
| =B13+2              |
| =B14+2              |

| London    | UK      |   230754 |
|-----------|---------|----------|
| Paris     | France  |   175880 |
| Paris     | France  |   168432 |
| Barcelona | Spain   |   125932 |
| Madrid    | Spain   |   110823 |
| Munich    | Germany |    99117 |
| Berlin    | Germany |   132812 |

| =SUBTOTAL(9,E9:E15)   |
|-----------------------|

| Withholding Taxes (by Region)   |
|---------------------------------|

| UK      | =(E9)*'Assump Withholding Tax'!C7       |
|---------|-----------------------------------------|
| France  | =(E10+E11)*'Assump Withholding Tax'!C9  |
| Spain   | =(E12+E13)*'Assump Withholding Tax'!C11 |
| Germany | =(E14+E15)*'Assump Withholding Tax'!C13 |

| =SUBTOTAL(9,E19:E22)   |
|------------------------|

| TOTAL (NET)   |
|---------------|

| =E16-E23   |
|------------|

| COSTS            |
|------------------|
| Band & Crew      |
| Sound Technician |
| Tour Coordinator |

| 8256                 |
|----------------------|
| 6904                 |
| =SUBTOTAL(9,E29:E30) |

| Hotel & Restaurants   |
|-----------------------|
| London                |
| Paris                 |
| Barcelona             |
| Madrid                |
| Munich                |
| Berlin                |

| 8388                 |
|----------------------|
| 15653                |
| 5445                 |
| 5113                 |
| 6369                 |
| 6592                 |
| =SUBTOTAL(9,E34:E39) |

| Other Costs       |
|-------------------|
| Agency Commission |
| Insurance         |
| Private Jet       |
| Transfer Cars     |
| Other             |

| =E16*0.11            |
|----------------------|
| 22024                |
| 341000               |
| 4237                 |
| 4819                 |
| =SUBTOTAL(9,E43:E47) |

| Total Costs   |
|---------------|

| =SUBTOTAL(9,E29:E48)   |
|------------------------|

| Net Income   |
|--------------|

| =E25-E50   |
|------------|

| Notes:                                                                                                                       |
|------------------------------------------------------------------------------------------------------------------------------|
| (1) Itinerary details are illustrative only.                                                                                 |
| (2) All entities are fictional. Geographies, assumptions, and amounts are illustrative and do not reflect any specific tour. |

| 2024 Fall Music Tour   |
|------------------------|

| Assumptions Related to Foreign Withholding Tax   |
|--------------------------------------------------|

| UK   | 0.2   |
|------|-------|

| France   | 0.15   |
|----------|--------|

| Spain   | 0.24   |
|---------|--------|

| Germany   | 0.15825   |
|-----------|-----------|

| Notes:                                                                    |
|---------------------------------------------------------------------------|
| (1) Use foreign withholding rates as noted on this tab for all reporting. |

| (2) Withholding on foreign earnings withheld at source for US persons and US entities, as per the   |
|-----------------------------------------------------------------------------------------------------|
| US Tax Treaties.                                                                                    |

| Costs Reported by Production Company   |
|----------------------------------------|
| 2024 Fall Music Tour                   |

| Description                   | USD   |
|-------------------------------|-------|
| COSTS                         | None  |
| Band & Crew (Fees & Per Diem) | None  |
| 10 members                    | 91000 |

| Hotel & Restaurants   |
|-----------------------|
| London                |
| Paris                 |
| Barcelona             |
| Madrid                |
| Munich                |
| Berlin                |

| 14232                |
|----------------------|
| 22296                |
| 8168                 |
| 8776                 |
| 12040                |
| 13226                |
| =SUBTOTAL(9,C11:C16) |

| Other Costs   |
|---------------|
| Petty Cash    |
| Car Service   |
| Fees          |

| 8000                 |
|----------------------|
| 2976                 |
| 1679                 |
| =SUBTOTAL(9,C20:C22) |

| Total Expenses   | =SUBTOTAL(9,C8:C23)   |
|------------------|-----------------------|

| Notes:                                                                                                        |
|---------------------------------------------------------------------------------------------------------------|
| (1) Itinerary details are illustrative only.                                                                  |
| (2) All entities are fictional. Geographies, assumptions, and amounts are illustrative and do not reflect any |
| specific tour.                                                                                                |