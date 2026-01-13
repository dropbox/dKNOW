## Networks of Necessity: Simulating COVID-19 Mitigation Strategies for Disabled People and Their Caregivers

Thomas E. Valles

∗ Hannah Shoenhard † Joseph Zinski † Sarah Trick

Mason A. Porter ∗§

Michael R. Lindstrom ∗¶

September 28, 2021

## Abstract

A major strategy to prevent the spread of COVID-19 is the limiting of in-person contacts. However, limiting contacts is impractical or impossible for the many disabled people who do not live in care facilities, but still require caregivers to assist them with activities of daily living. We seek to determine which interventions can prevent infections among disabled people and their caregivers. To accomplish this, we simulate COVID-19 transmission with a compartmental model that includes susceptible, exposed, asymptomatic, symptomatically ill, hospitalized, and removed/recovered individuals. The networks on which we simulate disease spread incorporate heterogeneity in the risks of different types of interactions, time-dependent lockdown and reopening measures, and interaction distributions for four different groups (caregivers, disabled people, essential workers, and the general population). Among these groups, we find that the probability of becoming infected is largest for caregivers and second largest for disabled people. Consistent with this finding, our analysis of network structure illustrates that caregivers have the largest modal eigenvector centrality among the four groups. We find that two interventions - contact-limiting by all groups and mask-wearing by disabled people and caregivers - most reduce the cases among disabled people and caregivers. We also test which group of people spreads COVID-19 most readily by seeding infections in a subset of each group and comparing the total number of infections as the disease spreads. We find that caregivers are the most potent spreaders of COVID-19, particularly to other caregivers and to disabled people. We test where to use limited vaccine doses most effectively and find that (1) vaccinating caregivers better protects disabled people than vaccinating the general population or essential workers and that (2) vaccinating caregivers protects disabled people about as much as vaccinating disabled people themselves. Our results highlight the potential effectiveness of mask-wearing, contactlimiting throughout society, and strategic vaccination for limiting the exposure of disabled people and their caregivers to COVID-19.

keywords: COVID-19, disabilities, networks, contagions, parameter estimation, vaccination

## Author Summary

Disabled people who need help with daily life tasks, such as dressing or bathing, have frequent close contacts with caregivers. This prevents disabled people and their caregivers from physically distancing from one another, and it also significantly increases the risk of both groups to contract COVID-19. How can society help disabled people and caregivers avoid infections? To answer this question, we simulate infections on networks that we model based on a city of about one million people. We find that one good strategy is for both disabled people and their caregivers to use masks when they are together. We also find that if only disabled people limit their contacts while other people continue their lives normally, disabled people are not effectively protected. However, it helps disabled people substantially if the general population also limits their contacts. We also study which vaccination strategies can most efficiently protect disabled people. Our simulations suggest that vaccinating caregivers against COVID-19 protects the disabled subpopulation

∗ Department of Mathematics, University of California, Los Angeles

‡ Assistant Editor at tvo.org (TVOntario)

† Department of Cell and Developmental Biology, University of Pennsylvania

§ Santa Fe Institute

¶ corresponding author: mikel@math.ucla.edu

‡

about equally effectively as vaccinating a similar number of disabled people. Our findings highlight both behavioral measures and vaccination strategies that society can take to protect disabled people and caregivers from COVID-19.

## 1 Introduction

The coronavirus disease 2019 (COVID-19) pandemic, which is caused by the severe acute respiratory syndrome coronavirus 2 (SARS-CoV-2) virus, has revealed major societal vulnerabilities in pandemic preparation and management [1]. Existing social disparities and structural factors have led to a particularly adverse situation for the spread of COVID19 in vulnerable groups. Therefore, it is crucial to examine how to mitigate its spread in these vulnerable groups [2] both to address these difficulties in the current pandemic and to prepare for future pandemics [3]. The effectiveness of society-wide behavioral interventions in mitigating viral spread in the general population is now well-documented [4-8]. However, the effectiveness of these non-pharmaceutical interventions (NPIs) has not been assessed in certain vulnerable groups. One such group is disabled people, who may choose to live in a group-care setting (such as a nursing home) or live independently with some caregiver support. It has been speculated that the latter arrangement increases the risk of disabled people to exposure to infections [9]. However, to the best of our knowledge, this situation has not been studied using epidemiological modeling. Vaccinations have also been extraordinarily effective at mitigating COVID-19; they have decreased case numbers and case rates, onset of symptomatic disease, hospitalizations, and mortality numbers and rates [10-16]. However, strategies for how to most efficiently use vaccines to protect independently housed disabled people have not yet been evaluated. In the present paper, we study a compartmental model of COVID-19 spread on a network to examine the effectiveness of several non-pharmaceutical interventions (NPIs) and vaccination strategies to prevent the spread of COVID-19 among independently housed disabled people and their caregivers.

People with disabilities who require assistance with activities of daily living (ADLs) may live in a long-term care facility or independently with some form of caregiving support [17,18]. Although extensive epidemiological and modeling studies have identified risk factors and mitigation strategies for COVID-19 outbreaks in long-term care facilities [19-24], there have not been similar studies of independently housed disabled people and their caregivers. Caregivers are often indispensable for the health and independence of disabled people because they assist with activities such as bathing, dressing, and using the bathroom. However, in a pandemic, public-health concerns dictate that it is important to minimize in-person contacts. Disabled people and their caregivers thus face an urgent question: How can they continue to interact while minimizing the risk of COVID-19 transmission?

This question is especially urgent because of the high prevalence of risk factors for severe COVID-19 in the disabled population. These risk factors, for which we give statistics for adults of ages 45-64 in the United States (see Fig. 1) [25,26], include obesity (about 46.7% of adults with a disability have a body mass index (BMI) that indicates obesity, compared with about 31.7% of adults without a disability), heart disease (15.0% of adults with a disability and 4.6% of adults without one), Chronic Obstructive Pulmonary Disease (COPD) (20.5% of adults with a disability and 3.7% of adults without one), and diabetes (25.6% of adults with a disability and 10.6% of adults without one). Additionally, whatever factor initially causes a person's disability can also complicate medical management of their case if they contract COVID19. Furthermore, isolating while ill can be impossible for disabled people because they rely on caregivers to assist them with essential daily tasks. This can make disabled people more prone to spread COVID-19 to caregivers if they contract it. Consequently, preventing COVID-19 infection among the disabled population and caregivers should be a high priority.

Caregivers also experience high risk of exposure to and death from COVID-19. Caregiving workers are disproportionately likely to be women, immigrants, and people of color. The median wage for in-home caregivers is $ 12.12 per hour, and their median annual earnings are $ 17,200 (which is below the U.S. federal poverty guideline for a two-person household) [27]. Experiencing poverty or being Black or Latinx independently increase the risk because of systemic disadvantages in accessing healthcare [28-30]. Furthermore, the COVID-19 pandemic has brought immense challenges to the caregiving workforce, including frequent lack of personal protective equipment (PPE), pandemic-specific training, paid time off, and childcare [27]. Finally, much caregiving work is impossible without close physical contact, which elevates caregivers' risk of occupational exposure. In summary, caregivers often belong to groups that are at higher risk both of COVID-19 exposure and of more severe illness from it.

According to a 2018 report [31], approximately 26% of U.S. adults (including about 41% of those who are 65 or older) have some form of disability. In 2016, Lauer and Houtenville [32] reported that 7.3% of the American population have a cognitive or physical disability that causes difficulty in dressing, bathing, or getting around inside the home (but we acknowledge the large uncertainty in this estimate). At least 2.4 million people in the U.S. (i.e., approximately 0.7% of the population) are employed as home-care workers, but this is likely an underestimate because of the difficulty of

## Comorbidities that increase COVID-19 risk

Fig 1. Rates of comorbidities that predispose individuals (of ages 45-64) to severe cases of COVID-19 among adults in the United States without (blue) and with (red) disabilities.

<!-- image -->

accurate statistical collection [27]. An intense time commitment and irregular hours are necessary for care, so many disabled people hire multiple caregivers and many caregivers work for multiple disabled people [33]. Therefore, there is significant potential for the spread of COVID-19 among and between these two vulnerable populations, making it a high priority to identify effective methods to reduce COVID-19 spread among disabled people and caregivers without compromising care.

To mitigate disease spread during a pandemic, governments may choose to implement society-wide shutdown orders, mask mandates, and/or physical-distancing guidelines. However, governments in some regions have been reluctant to issue such orders, and populations may not fully comply with them. This raises the issue of what disabled people and caregivers can do to protect themselves both with and without society-wide pandemic-mitigation efforts. With this in mind, we test how effectively mask-wearing (i.e., using PPE), limiting the number of caregiver contacts, and limiting contacts among disabled people prevent COVID-19 infections when the general population either maintains their normal contact levels or limits them. To the best of our knowledge, this is the first time that mathematical modeling has been used to evaluate these issues for COVID-19 infections.

Multiple COVID-19 vaccines are now widely available in some countries, but vaccine supplies remain scarce in other countries. As of late August 2021, only 1.6% of people in low-income countries have received at least one dose of any COVID-19 vaccine [34]. Furthermore, other pandemics may well emerge in the future. Consequently, it is valuable to evaluate how to most effectively allocate a small number of vaccine doses to protect vulnerable groups, such as disabled people. Specifically, we investigate whether vaccinating disabled people or caregivers is more effective than other vaccination strategies for reducing the total number of cases in these two vulnerable groups.

In this paper, we simulate COVID-19 spread on model networks that represent a city. We base the parameter values in these networks on Ottawa, Canada. Our stochastic model of disease spread takes into account several disease states (i.e., 'compartments'), different occupation types in a population, the heterogeneity of the risk across different interactions, and time-dependent lockdown measures. Our disease-spread model, which we explain in Section 2, allows us to quantitatively study our various questions under our set of assumptions. Using both computations of network structure and simulations of the spread of a disease on our networks, we find that disabled people and caregivers are both substantially more vulnerable to COVID-19 infection than the general population because of their large network centralities. We test the effectiveness of several NPIs - including limiting the number of social contacts, wearing masks, and limiting the number of caregivers that a given disabled person sees - at preventing COVID-19 spread among disabled people and their caregivers. By selectively seeding infections or blocking infections (via a simulated vaccine) in certain groups, we identify caregivers as major drivers of COVID-19 spread - especially among disabled

people and caregivers - and suggest that this group should be prioritized in vaccination campaigns.

Our paper proceeds as follows. We present our stochastic model of the spread of COVID-19 in Section 2, our results and a series of case studies in Section 3, and our conclusions and further discussion in Section 4. We describe the details of our model in Sections A and B of our Supporting Information.

## 2 A Stochastic Model of the Spread of COVID-19 Infections

We start by giving a rough idea of our stochastic model of the spread of COVID-19, and we then discuss further details in Section 2.2. Readers who are interested predominantly in the essence of our model can safely skip Section 2.2. We give a comprehensive list of our assumptions in Section 2.3. Readers who wish to use our code can find it at a Bitbucket repository. We previously wrote a white paper about this topic [35]; the present manuscript gives the full details of our study.

## 2.1 A Brief Overview of Our Model

Numerous researchers have used mathematical approaches to examine the spread of COVID-19 [36,37]. Such efforts have used a variety of frameworks, including compartmental models [38,39], self-exciting point processes [40,41] (which one can also relate to some compartmental models [42]), and agent-based models [43]. Many of these models incorporate network structure to examine how social contacts affect disease spread. Some models have incorporated age stratification [44], how mobility and other data can forecast the spread of COVID-19 [45-47], and/or the structure of travel networks [48]. In the present paper, we use an agent-based approach to study COVID-19 within a single city. Our approach involves simulating a stochastic process on time-dependent networks [49,50]. One of the features of our model is that different segments of the population have different degree distributions, with mixing between these different segments. To examine networks with these features, we use generalizations of configuration models [51,52].

In our model population, we consider three types of interactions between individuals, six disease states, and four distinct groups (i.e., subpopulations). We encode interactions using a network, and all interactions between different individuals involve exactly two people. We suppose that strong interactions describe interactions at home within family units (or, more generally, within 'household units'); weak interactions describe social interactions and interactions that take place at work, at a grocery store, and so on; and caregiving interactions describe interactions between caregivers and the disabled people for whom they care. We model each of these interactions with a different baseline level of risk of disease transmission. Weak interactions have the lowest baseline risk level, strong interactions have the next-lowest baseline risk level, and caregiving interactions have the highest baseline risk level.

We use a compartmental model of disease dynamics [53], which we study on contact networks [54,55]. We assume that our population (e.g., of a single city, like Ottawa) is closed and that each individual is in exactly one disease state (i.e., 'compartment'). Our model includes susceptible (S) individuals, who can contract COVID-19; exposed (E) individuals, who have the disease but are not yet infectious or symptomatic; asymptomatic (A) individuals, who do not have symptoms but can spread the disease; ill (I) individuals, who are symptomatically ill and contagious; hospitalized (H) individuals, who are currently in a hospital; and removed (R) individuals, who are either no longer infectious or have died from the disease. The A compartment includes prodromal infections, asymptomatic individuals, and mildly symptomatic individuals; in all of these situations, an individual has been infected, but we assume that they are not aware of it. Our model does not incorporate loss of immunity or births, and we classify both 'recovered' and removed individuals as part of the R compartment. In our study, an individual has been infected if they are no longer in the S compartment. Therefore, cumulative infections include every individual that is currently in the E, A, I, H, and R compartments.

We divide our model city's population into the following subpopulations:

- caregivers , who provide care to disabled people;
- disabled people , who receive care;
- essential workers , whose occupations prevent them from limiting contacts during lockdowns and similar policies, but who are not already included in the caregiving population; and
- the general population , which is everyone else.

2

Initial

Subpopulation

Strong Caregivers

Weak Caregivers

Disabled

Essential

General

2

2

Fig 2. An egocentric network (i.e., ego network) of an example disabled person on (A) day 43 (before the start of contact-limiting) and (B) day 45 during contact-limiting). The two ego networks encode contacts for the same disabled person. The label 'W 1 ' denotes the weak caregiver on day 43 and the label 'W 2 ' denotes the weak caregiver on day 45. (In this example, W 1 and W 2 are different caregivers. We illustrate the different groups in our model city (colors), interaction strengths between individuals (line thicknesses), and distances (numbers) from the ego. The edge weights are relative to the strong-contact weight of 1.

<!-- image -->

The individuals in the disabled subpopulation have two types of caregivers: weak caregivers, who are professional caregivers whose connections are likely to break if either individual in an interaction becomes symptomatic; and strong caregivers, whose caregiving relationship persists even if the individuals in it are symptomatic (and as long as neither individual is hospitalized). We consider these two types of caregivers to account for family members or close friends who always provide some care to a disabled person. Although our model includes a hospitalized compartment, we do not model doctors, nurses, custodial services, or other hospital staff who are involved in caring for COVID-19 patients. The caregivers in our model population refer strictly to individuals who provide supportive assistance to members of the disabled community in their homes. We also do not model skilled care facilities, such as nursing homes.

When an individual is symptomatic, we assume that they distance themselves (through so-called 'physical distancing' or 'social distancing') from society with a fixed probability b ∈ [0 , 1]. The probability can be less than 1 to account for a variety of situations, such as people who feel financial pressure to work anyway [56], people who have symptoms that are so mild that they are unaware of them, and people who ignore common decency. In our model, distancing by an individual who becomes ill means that they temporarily cut off their weak contacts or weak caregiver-disabled relationships and only maintain contacts within their household unit and possibly strong caregiver-disabled relationships until they recover. If an individual becomes hospitalized, these stronger contacts also break.

We seek to understand how COVID-19 spreads in these different groups over time and how different mitigation strategies, such as contact-limiting and mask-wearing, affect the outcomes. Consequently, we allow the distributions of the number of contacts to change with time and adjust the disease transmission probability to reflect the presence of masks.

We tune our baseline model to describe the city of Ottawa from its first reported case on 10 February 2020 [57] through its closure of non-essential businesses on 24 March 2020 [58] (the closure order occurred on 23 March) and then to understand how its 'Phase 1' reopening on 6 July 2020 [59] affected disease spreading. In Fig. 2, we illustrate an egocentric network (i.e., 'ego network') [60] that is centered at a single disabled person in the population before and after closure.

Distanced

## 2.2 Specific Details of Our Model

We now give a detailed description of our model. One of the key elements of the networks on which a disease spreads is their ability to describe the numbers and distributions of the contacts of different types of individuals. We do this by constructing networks using a generalization of configuration-model networks. See [61] for a review of configuration models.

To each node (i.e., individual) in a network, we assign a group (disabled, caregiver, essential worker, or general) and then assign both weak contacts and strong contacts. Additionally, we assign caregiver nodes to each disabled node and assign disabled nodes to each caregiver node. No individual is both a strong contact and a weak contact to the same person; no individual is both a caregiver for a disabled person and an ordinary weak contact for that disabled person, and so on. We anticipate a large variance in the number of weak contacts, with some people having many more contacts than others [62], so we assign each individual a number of weak contacts from an approximate truncated power-law distribution (see Section B). Because strong contacts represent household units, we assign each individual a number of strong contacts from an empirical distribution that we construct using census data of household sizes in Canada [63]. To model the pools of caregivers that are available to disabled people, we assume that each disabled node has a fixed number of weak caregivers (this pool does not change over time) and that this fixed number is the same for all disabled nodes. We were unable to find reliable data about the sizes of these pools, so we base the values of these quantities on educated guesses that are consistent with the lived experience of the disabled authors of the present paper. We also assign one strong caregiver to each disabled node. The contact structure in one of our networks can change over time. For example, weak contacts can break if a lockdown starts, both weak and strong contacts break when an individual is hospitalized, and so on. Each day, we choose one member of a disabled individual's caregiver pool uniformly at random to potentially provide care to them. (It is only potential care because that caregiver may have broken contact due to illness.) Each day, the disabled individual also receives care from a single strong caregiver, if possible. (This occurs as long as that contact has not been broken due to hospitalization.) In each time step, which consists of one day, the disease state (i.e., compartment) of an individual can change. From one day to the next, we compute the transition probability from susceptible to exposed using Eqs. (1) and (2) using each susceptible individual's disease state at the start of the day. On each day, we determine transitions between different disease states by generating exponential random variables for transition times. When a generated transition time occurs within a 1-day window, an individual changes compartments. If two different transitions are possible and both exponential random variables are less than 1 day, then we use the state transition that corresponds to the shorter transition time. Individuals who break their contacts because of illness do so immediately upon transitioning to a new compartment. Any network restructuring occurs at the start of a day (i.e., before we calculate exposure risks).

In the Supporting Information, we give the day-to-day evolution in Algorithms 1, 7, and 8 and the networkconstruction process in Algorithms 3, 4, 5, and 6. We host our code at a Bitbucket repository.

When we construct our networks, we use three families of discrete probability distributions.

- The distribution P ( a -, a + , µ ) is an approximate truncated power-law distribution. If N ∼ P ( a -, a + , µ ), then N takes integer values in { a -, a -+1 , . . . , a + } . For large n , we have that Pr( N = n ) = O (1 /n p ), where we choose p so that E ( N ) = µ . See our Supporting Information for full details.
- The distribution E ( p 0 , p 1 , p 2 , . . . , p k ) is a discrete distribution. If N ∼ E ( p 0 , p 1 , p 2 , . . . , p k ), then Pr( N = n ) = p n when n ∈ { 0 , . . . , k } and Pr( N = n ) = 0 otherwise.
- The distribution F ( k ) is a deterministic distribution that has only one attainable value. If N ∼ F ( k ), then Pr( N = n ) = δ n,k , where δ denotes the Kronecker delta (which equals 1 if the subscripts are equal and 0 if they are not).

Our model has three key dates: the first recorded case , which we set to be day 1 (i.e., 10 February 2020), as we use day 0 for initial conditions to produce a seed case of the disease (such cases are in the asymptmatic compartment); lockdown (i.e., 24 March 2020), which is when contact-limiting begins and some individuals wear masks; and reopening (i.e., 6 July 2020), which is when the city begins to reopen. For mask-wearing, we focus on four situations:

- None (N): nobody wears a mask;
- Disabled people and caregivers wear masks (D+C): disabled people and caregivers both wear masks when interacting with each other, but nobody else wears a mask;

- Disabled people, caregivers, and essential workers wear masks (D+C+E): all of the mask-wearing in the (D+C) scenario occurs, and we also assume that both individuals in an interaction wear a mask whenever there is a weak interaction with an essential worker (to model interactions in places like grocery stores, banks, and routine doctor visits); and
- All weak contacts wear masks (All*): the same individuals under the same conditions as in (D+C+E) wear masks, but we also assume that both individuals wear a mask in any interaction between weak contacts.

To model essential workers, we assume (except when there are symptoms of illness) that weak contacts with essential workers are not broken. Therefore, during a lockdown, essential workers continue to have a large number of weak contacts on average. We similarly characterize the caregiver subpopulation; they retain their interactions with their associated disabled nodes. We assume that an individual's weak contacts during lockdown are a subset of their weak contacts from before a lockdown. Upon a reopening, each individual is assigned a new number of weak contacts. They retain the weak contacts that they had during a lockdown, but they can also gain new weak contacts that they did not possess before the lockdown if their new number of weak contacts is larger than their number of contacts immediately prior to reopening. For example, if an individual has 3 weak contacts during lockdown and are assigned 7 weak contacts after reopening, then they need 4 weak contacts. These 4 new weak contacts can be different from that individual's weak contacts before the lockdown. We do this to account for situations (such as business closures or job loss) that cause individuals to visit different stores or workplaces after a city reopens.

We discretize time into units of ∆ T = 1 day. Our model, which one can view as an agent-based model, evolves as the individuals interact with other. Individuals who are in the S compartment can move into the E compartment, depending on their interactions on a given day. Each day, the probability that susceptible individual i remains in the S compartment is

<!-- formula-not-decoded -->

where B ( i ) is the set of all active (i.e., non-broken) contacts of individual i that are infectious, W ij is 1 if i and j are weak contacts and 0 otherwise, C ij is 1 if i and j have a caregiving relationship and 0 otherwise, and M ij (which can be equal to 0, 1, or 2) counts how many of individuals i and j wear a mask during an interaction between them. The term βw W ij w w C ij c m M ij / 2 gives the probability that node i becomes infected from an interaction with node j . Given σ , we compute the probability that i transitions from the susceptible compartment (S) to the exposed compartment (E) in a given day:

<!-- formula-not-decoded -->

We model the outcomes of transitions from E to A, transitions from A to I, transitions from A to R, transitions from I to H, transitions from I to R, and transitions from H to R as exponential processes with fixed rates of ν , α , η , µ , ρ , and ζ , respectively. In our simulations, transitions occur in intervals of size ∆ T (which we set equal to 1 day, as mentioned previously). When multiple transmissions between compartments are possible, such as from A to I and from A to R, we treat event transitions as competing exponential processes. (See Algorithms 1, 7, and 8 of the Supporting Information.) We summarize the possible state transitions and their rates in Fig. 3.

In our simulations, we uniformly-at-random initialize a fixed number A 0 individuals to be asymptomatic on day 0. To account for limited testing availability in the early stages of the epidemic, we assume that only a fraction τ of individuals in the I compartment (i.e., they are symptomatically ill but not hospitalized) have a positive COVID-19 test. Having a positive COVID-19 test means that the individual has a documented case of COVID-19. We determine whether an individual will test positive if they are symptomatically ill at initialization when we assign them a true/false flag P with probability τ for true. When P is true, if that individual becomes symptomatic (I), we suppose (for simplicity) that they have a positive COVID-19 test immediately upon moving into the I compartment (i.e., before the next day begins). When P is false, that individual only has a positive COVID-19 test if they are hospitalized. For simplicity, we again assume that their positive test takes place immediately upon moving into the H compartment. We also assume that we do not double-count individuals who have a positive COVID-19 test while in the I compartment if they later move into the H compartment. We assume that no asymptomatic infections are documented in the early spread of the disease. We compute daily tallies of cumulative documented cases at the end of each day. We need the assumption about not having positive COVID-19 tests of all infected individuals to be able to fit our parameters to the Ottawa data, which tabulates the number of documented cases (but not the cumulative number of total infections) over time.

In Table 1, we present the parameters that we use in our model. We discuss and support the values of these parameters in Section A of our Supporting Information. Whenever possible, we seek to infer parameters directly from

Fig 3. Schematic illustration of our compartmental model of disease transmission. Susceptible individuals (S), by being exposed to asymptomatic (A) or symptomatically ill (I) individuals, can become exposed (E) with a baseline transmission probability β . One can reduce the risk of an interaction through the NPI of mask-wearing; this multiplies the risk by the factor m 1 / 2 (if only one individual in the interaction wears a mask) or the factor m (if both individuals in the interaction wear a mask). Caregiving interactions have a higher risk (by a factor w c ) than the baseline and weak interactions have a lower risk (by a factor w w ) than the baseline. Exposed individuals are not yet contagious; however, these individuals eventually transition to the asymptomatic state. From the asymptomatic state, an individual can either become symptomatically ill or be removed (R), which encompasses recovery, death, and any other situation in which an individual is no longer infectious. If an individual is symptomatic, they can either be removed or become hospitalized (H). From the hospitalized state, an individual eventually transitions to the removed state. The state-transition parameters that we have not yet mentioned are fixed rates of exponential processes.

<!-- image -->

clinical data, instead of basing our parameters upon other models. (The exception is ν , which is the transition rate from the E compartment to the A compartment.)

## 2.3 Summary of our Assumptions

We now briefly summarize the main assumptions of our model

Population: Our model city's population is closed, so the city has no inflow or outflow.

Time units: We discretize time in units of ∆ T = 1 day.

Composition: The population of our city consists of the following types of individuals: 7 . 3% are disabled, 2 . 1% are caregivers, 14 . 72% are essential workers, and 75 . 88% are members of the general population. The roles of individuals do not change.

Disease compartments: Individuals can be susceptible, exposed, asymptomatic, symptomatically ill, hospitalized, or removed. All infected individuals must go through the exposed compartment before becoming infectious.

Transitions between compartments: We model an individual's daily infection rate through a probability of infection per interaction with a contagious individual, with interaction probabilities scaled up or down based on the types of interactions and the presence/absence of masks. All other transitions between compartments come from exponential processes that we compute one day at a time.

Strong contacts: We assign the numbers of strong contacts of all individuals from the empirical probability distribution D strong .

Weak contacts: According to an individual's assigned role and the status of contact-limiting, we determine the numbers of their weak contacts from an approximate truncated power-law distribution (see Section B) using D group,status , where group is one of 'gen/dis' (i.e., the general and disabled subpopulations), 'care' (i.e., caregivers), or 'ess' (i.e., essential workers) and status is one of 'pre' (i.e., not during a lockdown) or 'post' (i.e., during a lockdown). Recall

| Symbol         | Meaning                                                                                       | Value                                                | Reference   | Source        |
|----------------|-----------------------------------------------------------------------------------------------|------------------------------------------------------|-------------|---------------|
| f dis          | fraction of population who are disabled                                                       | 0 . 073                                              | [32]        | literature    |
| f care         | fraction of population who are caregivers                                                     | 0 . 021                                              | [64]        | literature    |
| f ess          | fraction of population who are es- sential workers                                            | 0 . 1472                                             | [64-66]     | literature    |
| f gen          | fraction of population who are part of the general population                                 | 0 . 7588                                             | [32,64-66]  |               |
| m              | risk-reduction factor from mask- wearing by both individuals in an interaction                | 0 . 34                                               | [7]         | literature    |
| b              | probability that an ill individual breaks its weak contacts                                   | 0 . 92                                               | [67]        | inferred      |
| w w            | weak edge weight                                                                              | 0 . 473                                              | [7,68]      | inferred      |
| w s            | strong edge weight                                                                            | 1                                                    | N/A         | by definition |
| w c            | caregiving edge weight                                                                        | 2 . 27                                               | [68,69]     | inferred      |
| β              | baseline transmission probability                                                             | 0 . 0112                                             | [68]        | inferred      |
| ν              | transition rate from E to A                                                                   | 1 day - 1                                            | [70]        | borrowed      |
| α              | transmission rate from A to I                                                                 | 0 . 0769 day - 1                                     | [71-74]     | inferred      |
| η              | transmission rate from A to R                                                                 | 0 . 0186 day - 1                                     | [71-74]     | inferred      |
| µ              | transmission rate from I to H                                                                 | 0 . 0163 day - 1                                     | [72,75-77]  | inferred      |
| ρ              | transmission rate from I to R                                                                 | 0 . 0652 day - 1                                     | [72,75-77]  | inferred      |
| ζ              | transmission rate from H to R                                                                 | 0 . 0781 day - 1                                     | [78]        | inferred      |
| τ              | probability of tested if ill but not                                                          | 0 . 04                                               | [57]        | fit           |
| C ∗            | hospitalized maximum number of contacts in power law                                          | 60                                                   | [57]        | fit           |
| D strong       | distribution of strong contacts                                                               | E (0 . 283 , 0 . 332 , 0 . 155 , 0 . 148 , 0 . 0816) | [63]        | literature    |
| D pool         | distribution of pool sizes                                                                    | F (10)                                               | n/a         | chosen        |
| D ess,pre      | essential worker weak-contact                                                                 | P (0 ,C ∗ , 21 . 37)                                 | [79]        | inferred      |
| D ess,post     | distribution when not distancing essential worker weak-contact distribution during distancing | P (0 ,C ∗ , 21 . 37)                                 | [79]        | inferred      |
| D gen/dis,pre  | general/disabled weak-contact                                                                 | P (0 ,C ∗ , 10 . 34)                                 | [79]        | inferred      |
| D gen/dis,post | distribution when not distancing general/disabled weak-contact                                | P (0 ,C ∗ , 7 . 08)                                  | [79]        | inferred      |
| D care,pre     | distribution during distancing caregiver weak-contact distribu-                               | P (0 ,C ∗ , 5 . 14)                                  | [79]        | inferred      |
| D care,post    | tion when not distancing caregiver weak-contact distribu-                                     | P (0 ,C ∗ , 4)                                       | [79]        | inferred      |
| A 0            | tion during distancing number of asymptomatic individ- uals on day 0                          | 341                                                  | [57]        | fit           |
| P Ottawa       | population of Ottawa                                                                          | 994837                                               | [80]        | literature    |

Table 1. The parameter values that we use in our study. In the 'Source' column, literature means that we found a value directly from data in the literature; inferred means that we inferred a value based on published data in the literature; by definition signifies a value that we set in our model formulation; chosen indicates that a value is unknown, but we made a choice in our model; borrowed indicates that we adopted a value directly from a model in the literature; and fit indicates that we use Ottawa case data along with other (i.e., not fit) parameters in this table to estimate a value.

that the weak-contact distribution has the same parameter values for disabled people and individuals in the general population.

Caregiving contacts: All disabled people have a pool of weak-contact caregivers of a size that is dictated by D pool . For each disabled person, we choose that pool uniformly at random from the set of caregivers. Additionally, each disabled person has one strong caregiver that we choose uniformly at random from the set of caregivers and they see that individual each day, unless either the disabled person or that caregiver is hospitalized.

Breaking contacts: Asymptomatic individuals do not break contacts (except in the form of contact-limiting). An ill (but not hospitalized) individual breaks their weak contacts with probability b . If an individual is hospitalized, they break both their weak contacts and their strong contacts until they move into the R compartment. An individual in the R compartment does not infect others with the disease; they may be deceased or simply no longer infectious. In our computations, those individuals regain their regular weak and strong contacts.

Interactions: Each day, an individual interacts with the same weak (except for caregiver-disabled interactions) and strong contacts unless the contact has been broken due to illness or when the contact distributions change. Each day, a disabled person interacts with their strong caregiver, unless illness prevents it. Each day, a disabled person interacts with a uniformly randomly selected member of their caregiver pool, unless illness prevents it. Even during contact-limiting stages, the weak contacts of essential workers do not break.

## 3 Results

We first compare the new daily documented cases and the cumulative number of documented cases in our model with empirical case data from Ottawa (see Fig. 4). We fit the parameters in our model up to May 10 (i.e., day 90) of the epidemic in Ottawa, and we assume that the city immediately enters a contact-limited phase on March 24 (i.e., day 44). We do the fitting (see Section A of the Supporting Information) by minimizing the glyph[lscript] 2 -error in the model's count of daily documented case counts. We show the 7-day mean of new daily documented cases; we calculate this mean over a sliding window that including the previous three days, the current day, and the next three days. At the endpoints, we truncate the window and take the mean over days that fill the window. We find reasonable agreement between the daily documented case counts in our model and the reported documented cases, but our match is not perfect. For example, the peak in daily documented case counts and the inflection point in cumulative documented cases occurs earlier in our model than it does in documented case records. This can arise from many possible factors, including delays in reporting cases (e.g., with differences on weekdays versus weekends), delays in the diagnosis of symptomatic individuals, changes of the model parameters (like testing availability) in time, or our use of only two degrees of freedom in our fits (with with most model parameters arising from sources that are not specific to Ottawa). The daily and cumulative documented case counts in our model deviate little from the data for the first 90 days, but our model subsequently tends to overestimate the case count. We speculate that this may stem from overestimating the number of contacts of the Ottawans. Our contact estimates come from survey data [79], which do not focus specifically on Ottawa. We wish to avoid overfitting, so we accept the fit performance.

Additionally, there is large variance in epidemic trajectories; that is, the 95% confidence window is large. We believe that one of the main factors behind this large variance is our use of an approximate truncated power-law distribution for weak contacts. If we replace these approximate truncated power-law distributions with deterministic distributions (i.e., distributions with 0 variance) with the same mean values, we obtain much smaller variances, but the disease also does not spread. We have chosen to use truncated power-law distributions to allow large variations in numbers of contacts, but this results in a larger variance. See Section C of our Supporting Information for further discussion.

Our baseline transmission probability β = 0 . 0112 is smaller than those that were employed in some other studies [44, 81], which used β ≈ 0 . 06. For the assumptions in our study, β = 0 . 0112 is appropriate. With β = 0 . 06, the disease is too infectious, and our simulations then result in a total documented case count that greatly exceeds the number of documented cases in Ottawa. This value of β = 0 . 06 is also inconsistent with secondary attack-rate studies [68] when they are combined with the durations that individuals spend in each compartment in our model. The fact that the disease can still spread so effectively with β = 0 . 0112 perhaps stems from our network structure, as some individuals can be superspreaders.

3000 -

&amp; 2500

Ca:

Cumulative Documented

2000 -

1500 -

1000 -

500 H

10 Feb. 2020

Stochastic-Model Mean Versus Ottawa Data (100 Simulations)

Stochastic-Model Mean

•*****• Ottawa Data

95% of Simulations

Calibration Ends

24 Mar. 2020

10 May 2020

Date

10 May 2020

Date

Fio 4. Comnarison of a mean of 100 simulations of our stochastic model of COVID-19 spread with (left) cumulative

Fig 4. Comparison of a mean of 100 simulations of our stochastic model of COVID-19 spread with (left) cumulative documented case counts and (right) the 7-day mean of new daily documented cases. We calculate the 7-day mean is over a sliding window that includes the previous three days, the current day, and the next three days. We fit the parameters by minimizing the glyph[lscript] 2 -error of the model's predictions of daily case counts over the first 90 days. We show the mean of our model in blue and the Ottawa case data in red. The gray window indicates the middle 95% of these 100 simulations. On day 44 (24 March, 2020), all subpopulations limit contacts and the (D+C+E) mask-wearing scenario begins. The graphs terminate on day 148, when Ottawa had its first reopening.

<!-- image -->

Lockdown Strategies Enacted

• 50

45

40

35

30

25

20

15

10

5

New Daily Documented Cases (7-Day Mean)

10 Feb. 2020

7 Jul. 2020

24 Mar. 2020|

7 Jul. 2020

А 120-

100-

80

60

40

Number of Neighbors

20-

70-

60-

50

20-

10-

B

ors

2500

Contact - Limitir

• Initial

To help us understand the results of simulating our stochastic model of COVID-19 spread, we examine the structural characteristics of the networks on which we perform our simulations. Because different types of contacts have different levels of disease transmission, we base our measures on the statistics of weighted networks. Additionally, our network contact structure changes with time. For one network from our network model, we compare two days - one before contact-limiting and one during it. For each of the two networks, we compute the number of first-degree contacts (i.e., direct contacts), second-degree contacts (i.e., contacts of direct contacts), contact strength (i.e., edge weight, which we interpret as the 'conductance' of a disease across a contact), and eigenvector centrality (i.e., the leading eigenvector of the network's adjacency matrix, where larger values of eigenvector centrality are associated with 'hightraffic' individuals, who are visited often by a random walker on the network [60,62]). We are interested in eigenvector centrality because the largest eigenvector-centrality value in a network plays a role in determining that network's susceptibility to a widespread outbreak of a disease under certain conditions [82]. We consider the distribution of the eigenvector centralities for different subpopulations in our model city. We find that caregivers have the most first-degree and second-degree contacts (see Fig. 5A,B) and the largest mean strength (see Fig. 5C). Essential workers have the largest mean eigenvector centrality (because of the heavy-tailed distribution of their contacts), whereas caregivers have the largest modal eigenvector centrality (see Fig. 5D). We also test the effects of contact-limiting and mask-wearing (i.e., PPE status) strategies on the strengths and eigenvector centralities of various populations. Both NPIs reduce the strength of a node, and contact-limiting in particular reduces the heavy tails of the distribution of strengths for caregivers, disabled people, and members of the general population (see Fig. 6). In other words, contact-limiting reduces the probability that individuals have a large number of contacts. Caregiver Disabled

Subpopulation

Subpopulation

Fig 5. Characterization of centrality measures of subpopulations in the networks on which we run our stochastic model of COVID-19 spread. The violin plots depict empirical probability densities. The initial situation, for which we show day 43 of one simulation, has no contact-limiting. The distanced situation, for which we show day 45 of the same simulation, has contact-limiting in all subpopulations. For each subpopulation, we calculate distributions of (A) the number of neighbors (i.e., direct contacts), (B) the number of second neighbors (i.e., contacts of contacts), (C) the strength of the contacts with neighbors, and (D) eigenvector centrality.

<!-- image -->

50

1.00

40-

0.75-

=30-

0.50-

С 20.

0.25-

Eigenvector Centrality

10-

0.00

•E Status

OPE Status onulation

opulation

N

Contact - Limiting

• Initial

Distanced

Statistic

<!-- image -->

N

Fig 6. Characterization of the effects of mask-wearing on centrality measures of subpopulations in the networks on which we run our stochastic model of COVID-19 spread. The violin plots depict empirical probability densities. The initial situation, for which we show day 43 of one simulation, has no contact-limiting. The distanced situation, for which we show day 45 of the same simulation, has contact-limiting in all subpopulations. We modify edge strengths by supposing that masks have the effectiveness that we indicated in Table 1. To indicate the mask-wearing statuses of different scenarios, we use the notation that we defined in Section 2.2. For each subpopulation, we compute (A) the edge-weight distribution and the (B) eigenvector-centrality distribution.

We also test how much different contact-limiting and mask-wearing strategies affect the different subpopulations in our model. We consider different mitigation strategies, which we assume are deployed on day 44, and we compare the of cumulative infections on day 148 for these strategies. We consider the mask-wearing strategies that we outlined in

1.0

0.6

0.2

0.0

Fraction Infected Through Day 148

General

None

Essential Caregiver

Subpopulation

Fig 7. Mean cumulative infections in the general population (blue), essential workers (purple), caregivers (gold), and disabled people (red) for different contact-limiting and mask-wearing statuses. The mask-wearing statuses are the same as in Fig. 6.

<!-- image -->

Section 2.2 and the following three contact-limiting strategies:

- No contact-limiting: All people maintain their contacts for the entirety of the 148 days.
- Only disabled people limit their contacts. Disabled people reduce their number of weak contacts on day 44. All other populations maintain their contacts.
- Everyone except for essential workers limits contacts. All subpopulations other than essential workers reduce their number of weak contacts on day 44.

We first consider the optimistic scenario in which all weak interactions involve mask-wearing. In this case, when everyone limits contacts on day 44, our simulations yield a mean of 13,242 cumulative infections through day 148. This is approximately 11.2% lower than the 14,910 cumulative infections through day 148 when only caregivers, disabled people, and weak interactions with essential workers wear masks. We conclude that universal mask-wearing (specifically, in all situations except within households) is an effective NPI for reducing the number of COVID-19 cases. For all of our subsequent simulations, we assume that weak contacts wear masks only in interactions that involve essential workers , unless otherwise noted.

We find that contact-limiting by only the disabled subpopulation has a relatively small effect on the number of their cumulative infections; it reduces the percent of them who become infected from 52.3% to 43.1%. Contact-limiting by only disabled people yields a similar result for caregivers, with a reduction in the percent of infected caregivers from 70.5% to 62.4%. Contact-limiting by all subpopulations has a larger effect; it reduces the percent of infected individuals in the disabled subpopulation to 21.0% and that of caregivers to 32.5%. Mask usage in both the disabled and the caregiver subpopulations protects both subpopulations even in the absence of any contact-limiting. The percent of disabled people who become infected decreases from 52.3% to 35.8%, and the percent of caregivers who become infected decreases from 70.5% to 40.3%. When essential workers, caregivers, and disabled people all wear masks, this protection is enhanced. The percent of disabled people who become infected decreases to 16.9%, and the percent of caregivers who become infected decreases to 19.5%. Finally, when all weak contacts wear masks, 2.7% of disabled people and 3.8% of the caregivers become infected. When all subpopulations limit contacts and all subpopulations wear masks (except within a household), 1.8% of the disabled subpopulation and 2.7% of the caregiver subpopulation become infected. We summarize our results of the mask-wearing interventions in Fig. 7.

Because COVID-19 guidelines recommend reducing the number of contacts between individuals, we test whether reducing the number of weak caregiver contacts per pool - while maintaining daily caregiving interactions - helps protect disabled people and/or caregivers. This NPI affects the total number of contacts of disabled people, but it does not reduce the total amount of time that they are exposed to these contacts. We test caregiver pool sizes of 4, 10, and 25, and we find that reducing caregiver pool size does not reduce infections among caregivers or disabled people (see Fig. 8).

D+C

D+C+E

All*

B

Fraction Infected

0.04

0.03

0.02

0.01†

0.00

0.04

0.03

0.02 +

0.01 †

0.00

General

Caregiver

4

10

25

Number of Caregivers Per Pool

Essential

Disabled

Fig 8. Effects of the number of caregivers (4, 10, or 25) that are assigned to a given disabled person on the mean fraction of each subpopulation that becomes infected. The label 'DCE PPE' refers to the (D+C+E) mask-wearing scenario.

<!-- image -->

Fig 9. Effects of (A) the probability of breaking weak contacts when will and (B) mask effectiveness on the mean fraction that each subpopulation becomes infected.

<!-- image -->

In our investigation, we are particularly uncertain about the values of two parameters: the probability that individuals break weak contacts when they become ill and the effectiveness of masks. Therefore, we repeat our simulations with otherwise baseline conditions for different values of these parameters (see Table 1). We choose the values of m as the boundaries of the 95% confidence window in mask effectiveness in [7].We choose the values of b as educated guess as to reasonable best-case and worst-case scenarios. As expected, reducing the probability of breaking weak contacts when ill (see Fig. 9A) and reducing mask effectiveness (see Fig. 9B) both increase the number of infections. Importantly, however, varying these parameters does not affect the overall pattern of infections; in particular, caregivers remain the most susceptible subpopulation.

Having observed that caregivers are the most likely to be infected among all subpopulations across all tested parameter sets, we investigate whether caregivers are also the most prone to spreading COVID-19. To do this, we seed all initial infections entirely in a single subpopulation, rather than distributing the initially infected individuals uniformly at random across the entire population. We calculate the mean fraction of each subpopulation that was infected cumulatively through 148 days. We find that the caregivers are the most potent spreaders of COVID-19, with each subpopulation reaching its highest infection rate when only caregivers are infected initially (see Fig. 10). Seeding all initial infections among only disabled people leads to the second-largest number of infections in the caregiver subpopulation.

As we explain in our Supporting Information (in Section A), because of the intimacy of caregiver-disabled interactions, the relative risk of such an interaction is likely higher than is the case for typical household interactions. In Section C of our Supporting Information, we also consider w c = 1 (i.e., the risk of a caregiver-disabled interaction is the same as a household interaction) w c = 1 . 5 (i.e., the risk of a caregiver-disabled interaction is only moderately higher than a household interaction). When w c = 1, essential workers are the most potent disease spreaders to all subpopulations except for the spread of the disease from caregivers to other caregivers. However, when w c = 1 . 5, caregivers are the most potent disease spreaders to the disabled community and to themselves, and essential workers are the most potent disease spreaders to the general population and to themselves. This suggests that our conclusions about the impact of the caregiver subpopulation on the disabled subpopulation are plausible even if the relative risk w c is only moderately larger than 1.

Our finding that caregivers are the subpopulation that is most prone to spreading COVID-19 has potential impli-

Subpopulation Infected Through Day 148

Caregiver

Disabled

Essential Worker

General

Fractions Infected Through Day 148

0.02346

0.04102

0.01606

0.01891

0.01083

0.02793

0.03558

0.01823

General

Essential Worker

Subpopulation Initially Infected

0.05635

0.07839

- 0.07

Fig 10. Fraction of each subpopulation that is infected through day 148 when all of the initially infected individuals are in a single subpopulation. On day 44, all groups limit contacts and the (D+C+E) mask-wearing scenario begins.

<!-- image -->

cations for vaccine prioritization because vaccinating caregivers can indirectly protect other subpopulations. Because initial vaccine supplies are often extremely limited, we test the efficacy of providing vaccination to only a small fraction of the total population. To do this, we simulate the distribution of a very limited amount of vaccine - equivalent to half (i.e., 10,151) of the mean remaining susceptible caregivers on day 148 (this is equal to approximately 1% of the total city population) - by moving a uniformly random subset of either susceptible caregivers, disabled people, essential workers, or the general population immediately to the removed compartment. When there were fewer susceptible people in a subpopulation than people to move, we move everyone in that subpopulation (and no other individuals) to the removed state. We also simulate a scenario with no vaccination. We simulate reopening at the same time as vaccination. In a reopening, all subpopulations return to their original weak-contact distributions, but all people wear masks during all non-household interactions. For a timeline, see Fig. 11A. We then simulate our stochastic model of infections until day 300 and calculate the number of infections that are potentially preventable through the above vaccination strategies by comparing the results of these simulations to simulations that do not incorporate vaccination. This enables us to evaluate the benefits that vaccinating each subpopulation confers indirectly to other subpopulations.

Consistent with our previous findings, vaccinating caregivers prevents the largest number of infections. In our simulated scenario, targeting limited vaccinations to the caregiver subpopulation leads to a drop in total infections of 8.7% in comparison to the scenario without vaccinations (see Fig. 11B,C). It is second-most effective to vaccinate essential workers (this prevents 4.1% of the total infections) and third-most effective to vaccinate the disabled subpopulation (which prevents 3.4% of the total infections). Vaccinating the same number of individuals in the general population prevents only 0.7% of the total infections. It is possible for the number of prevented infections to be smaller than the number of people who are vaccinated because not all vaccinated individuals would have become infected if they were not vaccinated. One plausible scenario in which this can occur is if the disease prevalence is relatively low in the subpopulation that is vaccinated, as is the case with the general population in our simulation.

Vaccinating caregivers is an effective strategy to protect disabled people. When 10,151 caregivers are vaccinated, we reduce infections in disabled people by a mean of 17.7%. Vaccinating the same number of disabled people spares a mean of 17.9% of the disabled subpopulation (i.e., almost an equal number) from infection. These almost equal percentages may arise from the relative sizes of the caregiver and disabled subpopulations in our model. Vaccinating 10,151 individuals entails vaccinating exactly half of remaining susceptible caregivers, but 10,151 individuals constitutes only about 14% of the disabled subpopulation. Therefore, when the number of vaccines is extremely limited, vaccinating caregivers may be comparably effective at protecting the disabled population as directly vaccinating disabled people.

Notably, vaccinating either the caregiver or the disabled subpopulation is much more effective at protecting the disabled subpopulation than vaccinating the essential-worker subpopulation, which only protects 1.1% of the infections

argeted vac

B

200

ing L

Total Infections Avoided

In 152 Days Post-Vaccine

C

152 Days

General

Subpopulation Partially Vaccinated

General

Essential

Essential

Subpopulation Partially Vaccinated

Subpopulation

General

Essential

Fig 11. Number of infections that are prevented in each subpopulation when each subpopulation is vaccinated with the indicated number of vaccines.

<!-- image -->

in the disabled subpopulation. Vaccinating caregivers also spares slightly more members of the general population than vaccinating essential workers; about 5.8% of the general-population infections are prevented when 10,151 caregivers are vaccinated, and about 3.4% of general-population infections are prevented when 10,151 essential workers are vaccinated. In our case study, the essential-worker subpopulation is the only subpopulation for whom the best strategy (among those that we considered) is to vaccinate the essential-worker subpopulation. With this strategy, they prevent 9.6% of the infections, which is better than the 5.5% that they prevent when the caregiver subpopulation is vaccinated (see Fig. 11C).

In our case study, we find that vaccinating the disabled subpopulation does not protect the caregiver subpopulation as effectively as vaccinating caregivers protects the disabled subpopulation. When 10,151 disabled people are vaccinated, a mean of about 11.0% of the caregiver cases are prevented. When the same number of caregivers are vaccinated instead, about 55.5% of the caregiver cases are prevented (see Fig. 11). This fivefold difference may arise from the relative sizes of the caregiver and disabled subpopulations. Because a relatively small fraction of the disabled people with whom any given caregiver interacts will be vaccinated and caregivers are often in the pools of multiple disabled people, our case study suggests that caregivers' risks are not mitigated greatly when only a small fraction of the disabled subpopulation is vaccinated.

## 4 Discussion

We now summarize and discuss our key results.

## 4.1 Our Most Significant Findings

The caregivers and disabled subpopulations are extremely vulnerable to COVID-19 infection . We simulated the spread of COVID-19 on networks to evaluate how vulnerable four interconnected populations - caregivers, disabled people, essential workers, and the general population - are to infection. Across multiple simulation conditions, we found that caregivers have the highest risk of infection and that disabled people have the second-highest risk of infection. This observation arises from multiple structural factors in our contact networks. First, there are many fewer caregivers than disabled people, so each caregiver typically has contact with multiple disabled people. This is reflected by caregivers having the largest number of direct neighbors and neighbors of neighbors. Second, caregiver-disabled connections are stronger than other connections, which - along with the large number of direct connections of caregivers contributes to caregivers also having the edges with the largest mean weights. Third, some of our simulations involved a contact-limiting phase, in which individuals reduce their number of weak connections. However, caregiver-disabled contacts do not break during this phase. These structural factors render caregivers and disabled people particularly vulnerable to infection with COVID-19. We also found that caregivers are the most potent spreaders of COVID-19 once they are infected, and we suggest that this is due to the same factors (specifically, being well-connected in a social newtork) that make them most vulnerable to becoming infected. This agrees with the observations of Gozzi et al. [83], who examined two different spread-limiting strategies in an activity-driven network model and found that the most active nodes that do not comply with a spread-limiting strategy are the major drivers of disease spread. Reassuringly, our findings are robust to changes in the parameters - the effectiveness of masks and the probability of breaking contact when ill - in which we had the greatest uncertainty.

In our model, we assumed that the transition probability to become hospitalized is the same for all subpopulations, and we did not model death. However, disabled people are more likely than other individuals to have medical conditions that predispose them to severe cases of COVID-19 and accessibility barriers to receiving healthcare [84] and caregivers are more likely to belong to marginalized groups that are at increased risk due to systemic structural barriers in accessing medical care. Taking these factors into account may reveal an even more disproportionate disease burden on caregivers and disabled people. Ortega-Anderez et al. [85] observed that small decreases in the exposure of medically vulnerable subpopulations significantly decrease overall mortality, underscoring how critical it is to identify interventions that effectively protect caregivers and disabled people.

Effective interventions . It is essential that the necessary medical services that at-home caregivers provide to disabled people continue to be available during a pandemic. These services are essential for survival; going without caregiving services endangers a disabled person's health. Therefore, we tested the effectiveness of various NPIs at preventing the spread of COVID-19 among these subpopulations. We found that mask-wearing during contact between caregivers and disabled people is a very effective strategy for reducing infections in both subpopulations. This finding

agrees with recent agent-based [86,87] and bond-percolation [88] models of mask-wearing interventions. We recommend that home-healthcare agencies provide their employees with masks and (whenever possible) mandate their use on the job.

Additionally, we found that contact-limiting by disabled people alone only slightly reduces their risk of contracting COVID-19 if it is not accompanied by contact-limiting in the rest of the population. When all groups limit contacts, cases among disabled people and caregivers fall by almost 50%. This result underlines the fact that changes in behavior in the general population can drive changes in disease spread in the disabled subpopulation. Disabled people alone are not numerous enough to change large-scale epidemic dynamics with their behaviors, and they are vulnerable to increases in disease spread that can occur when the general population changes its behavior. In the context both of the current COVID-19 pandemic and possible future pandemics, we emphasize the critical influence of behavior by the general popular on disabled communities. Mitigation efforts by the general population, such as contact-limiting (as in the present study), can protect disabled people much more than interventions in only the disabled subpopulation.

Vaccinating caregivers shields other subpopulations, including disabled people. A major application of modeling of the spread of a disease on a network is evaluating strategies for targeted vaccination [54, 55, 89]. Prior research suggests that, under certain conditions, the largest eigenvector centrality of a network helps determine a network's threshold (e.g., in the form of a basic reproduction number) for a widespread outbreak of a disease [82]. This suggests that vaccinating nodes with large eigenvector centralities may be an effective control strategy. Several COVID19 vaccines have received rigorous safety testing for approval across the globe [90-93], and we sought to determine the most effective vaccination strategy in the context of our model. As a first step, we calculated eigenvector centralities of the nodes in the network's four subpopulations. We calculated that essential workers have the largest mean eigenvector centralities in a single simulated population and that caregivers have the largest modal eigenvector centralities in the same simulated population. The difference between the mean and modal values is a direct consequence of the contact distributions of these two subpopulations. For example, essential workers are sometimes in very large workplaces and sometimes in very small workplaces, whereas caregivers almost always work with multiple disabled people.

Investigating network structure alone in our model did not resolve which subpopulation is the most efficient one to vaccinate. Therefore, we analyzed how the dynamics of disease spread were affected by selectively vaccinating a subset of each of these subpopulations. We considered a hypothetical vaccine that is completely effective and permanently prevents any individual who receives it from contracting or spreading the virus SARS-CoV-2. Although this is unrealistic vaccinated people can still contract SARS-CoV-2 and even spread it to others [94] - vaccinated people are much less likely than unvaccinated people to be diagnosed with the disease COVID-19 [95]. They also experience a faster drop in viral load when they are infected, so transmission periods may be shorter in vaccinated people [96].

Vaccine effectiveness against household transmission of so-called 'breakthrough cases' of COVID-19 infection in vaccinated individuals was estimated at 71% in one study [97]. However, this study was conducted when the Alpha variant (Pango lineage designation B.1.1.7) of SARS-CoV-2 was predominant, and it is unknown whether this finding holds for the Delta variant (Pango lineage designation B.1.617.2). As new variants emerge frequently and vaccine adherence, availability, and manufacturers vary worldwide, we chose to examine a simplistic scenario instead of attempting to model any specific real-world situation.

We measured the effectiveness of vaccine strategies by comparing the number of infections in scenarios with and without vaccination (without minus with). This number includes both infections that are prevented directly (specifically, when an individual who would have become infected had already received the vaccine) and ones that are prevented indirectly (specifically, chains of transmission that did not occur because individuals who would have spread the virus were instead vaccinated against it). Our simulations suggest that vaccinating caregivers (1) prevents the largest total number of infections and (2) prevents the most infections in three of the four subpopulations. (The exception is the subpopulation of essential workers.) In our simulations, vaccinating a specified number of caregivers protected an almost equal number of disabled people from infection (because of indirect prevention) as vaccinating the same number of disabled people.

It is necessary to be cautious when interpreting our findings about the relative efficiency of vaccinating various subpopulations. To obtain our results, we assumed that vaccines prevent the spread of COVID-19 from a vaccinated individual to other individuals. In the extreme hypothetical scenario where vaccines prevent serious illness but have no effect on viral transmission from vaccinated individuals, it is likely better to employ them in populations (e.g., disabled people) that are more likely to experience hospitalization and death. Moreover, even if vaccinating caregivers does turn out to be the most efficient way to reduce total case numbers of COVID-19, it may still be more ethical to prioritize vaccinating individual disabled people, particularly those who are elderly or have conditions that predispose them to

severe disease [85]. In the real world, vaccination campaigns must balance many factors - including medical risk, public health, and equity - when assigning priority [98]. Additionally, we reiterate that the precise conclusions about vaccination strategies from our model may not hold in real-world scenarios. For example, it is important to consider a variety of local factors, including the amount of vaccine that is available, the relative sizes of the caregiver and disabled populations, and the distributions of age and pre-existing conditions in these populations.

When a small number of caregivers serve a large number of disabled people who are not at particularly high medical risk, vaccinating caregivers has several benefits: (1) it protects caregivers, who often are in demographic groups with an elevated risk of COVID-19 complications, for their own sake; (2) it indirectly shields the disabled people for whom they care; and (3) it prevents the disruption of essential caregiving services to disabled people when caregivers are infected and must quarantine. Furthermore, for disabled people who cannot gain the benefit of vaccination - whether due to access issues with transportation or at vaccination centers, immunosuppression, or other health challenges - our findings suggest that vaccinating caregivers can be an extremely useful preventative strategy.

Our model strongly suggests that caregivers of disabled people are at increased occupational risk of both contracting and spreading COVID-19 and that protecting caregivers also provides substantial, quantifiable benefits to the vulnerable population that they serve. Therefore, we suggest that caregivers should be among the groups that are given the opportunity to receive a vaccine as a high priority.

Especially when vaccines are not readily available, we emphasize the importance of continuing effective NPIs, such as mask-wearing and contact-limiting, in all subpopulations (including the general population). Additionally, vaccination campaigns should make it a priority to protect disabled people, and they should consider early vaccination of caregivers and disabled people as one potential strategy among continued society-wide NPIs to accomplish this goal.

## 4.2 Limitations of our Study

In interpreting our results, we made many assumptions to construct a tractable model to study. Accordingly, our results occur in the context of a variety of hypotheses about the epidemiology of COVID-19 in the disabled community and optimal strategies to mitigate the spread of the disease. Although we consider our hypotheses to be reasonable ones, we obviously did not perfectly describe the complexity of COVID-19, how it spreads, and how human behavior affects its spread. (See [99] for a recent review agenda for integrating social and behavioral factors with disease models.)

In reflecting on our assumptions and our modeling (of both network structure and the spread of COVID-19), there are a variety of natural steps to take to enhance our work. Although they are beyond the scope of the present paper, we elaborate on some of them. We encourage careful examination of the following ideas:

- Incorporating skilled nursing facilities and hospitals: We assumed that caregivers provide at-home care to disabled people. There are many disabled people who live in skilled care facilities, which have a different network of caregiving and care-receiving than the ones that we examined. One can perhaps argue that we modeled these effects indirectly through the heavy-tailed distributions of weak contacts, but that does not incorporate the intricacies of these healthcare settings.
- Lack of entry into and exit from a city: We did not consider the possibility that people enter a city, which can introduce more infections. This type of effect was studied in [48].
- Uncertainty in the numbers of disabled people and caregivers in a population: There is a lot of uncertainty in the proportions of disabled people and caregivers in a population. Unfortunately, there is not much reliable information about how many disabled people receive assistance for their activities of daily living and how many people in society serve as caregivers (possibly in an unpaid or undocumented capacity). It is very important to obtain more data about this and to incorporate it into modeling efforts.
- More precise distributions of contacts: It was very difficult for us to estimate the contact distribution of people before and during a lockdown, and it was even more difficult to estimate the level of contact-limiting. It is worthwhile to study the effects of different types of distributions of weak contacts. We briefly explore this issue in Section C of the Supporting Information.
- Incorporating daily randomness of interactions: During each phase of our model COVID-19 pandemic, we fixed the set of potential daily contacts (they are only potential contacts because illness can temporarily sever ties) of the population's individuals, except for interactions between disabled people and caregivers, for whom we assigned a random caregiver from a pool to each disabled person.

- Modeling contact changes during a city's reopening: One limitation of our network model is that when we assigned additional contacts to individuals after our model city reopens, we did so in a random way (for simplicity), rather than having individuals resume the contacts that they had before a lockdown. This choice mixes the contacts in the network, and studying the consequences of this choice seems important.
- Heterogeneity in mask effectiveness: We assumed that all masks give the same transmission-reduction benefits. However, this is not realistic. There are a large variety of mask types and some people do not wear masks correctly, so it seems worthwhile to examine how heterogeneity in mask effectiveness affects disease dynamics.
- Modeling mask-compliance probabilistically: We assumed that masks are either worn or not worn by an entire subpopulation for given types of interactions. In reality, only some fraction of a subpopulation will wear masks.
- Studying the importance of caregivers to disease spread: We speculated that the larger modal eigenvector centralities of caregivers leads to COVID-19 spreading more effectively from caregivers than from other subpopulations. It seems useful to study the importance of caregivers to disease spread from a theoretical perspective. For example, given the properties of the contact distributions of different members of a society, it is desirable to investigate the probability with which a large-scale epidemic occurs and how quickly it spreads in its early stages if only caregivers are infected when it starts.
- Modeling vaccination outcomes: Our model simplistically assumes that vaccination fully prevents COVID-19 infection and transmission. In reality, vaccination provides robust but incomplete protection from COVID-19. Vaccinated individuals can experience asymptomatic or symptomatic disease and can transmit the virus to others, although at lower rates than unvaccinated individuals [12]. Our model does not take into account infection of or transmission by vaccinated individuals.
- Effects of new variants of SARS-CoV-2: The SARS-CoV-2 virus has mutated over time, and some of our parameter estimates may depend on specific strains of the virus and differ across both time and geographic regions.
- Uncertainties in timing: We used the simplistic assumption that all positive tests of COVID-19 of individuals in the I and H compartments occur at the beginning of an individual's first day in the relevant compartment. We also assumed that the availability of COVID-19 tests was the same throughout the first 148 days of the COVID-19 pandemic. Neither of these assumptions is realistic, and it seems worthwhile to consider more realistic testing scenarios.

## 4.3 Conclusions

We constructed a stochastic compartmental model of the spread of COVID-19 on networks that model a city of approximately 1 million residents and used it to study the spread of the disease in disabled and caregiver communities. Our model suggests that (1) caregivers and disabled people may be the most vulnerable subpopulations to exposure in a society (at least among the four subpopulations that we considered); (2) mask-wearing appears to be extremely effective at preventing infections in caregivers and disabled people; (3) contact-limiting by an entire population appears to be far better at protecting disabled people than contact-limiting only by disabled people; and (4) caregivers may be the most potent spreaders of COVID-19 and vaccinating caregivers can be extremely helpful to protect disabled people.

## Acknowledgements

We gratefully acknowledge Deanna Needell and Sherilyn Tamagawa for making the introductions that allowed our team to form, and we thank Stephen Campbell (Data and Policy Analyst at PHI) for directing us to helpful resources and helping refine our questions. MAP acknowledges support from the National Science Foundation (grant number DMS-2027438) through the RAPID program.

## Competing interests

HS provides care, and JZ and ST receive care. ST reports on the COVID-19 pandemic as a journalist.

## References

1. Maxmen A. Has COVID Taught us Anything About Pandemic Preparedness? Nature. 2021;596:332-5. Available from: https://www.nature.com/articles/d41586-021-02217-y .
2. Tackle Coronavirus in Vulnerable Communities. Nature. 2020;581(7808):239-40. Available from: http://www. nature.com/articles/d41586-020-01440-3 .
3. Zelner J, Masters NB, Naraharisetti R, Mojola S, Chowkwanyun M. There Are No Equal Opportunity Infectors: Epidemiological Modelers Must Rethink Our Approach to Inequality in Infection Risk; 2021. arXiv:2109.00580. Available from: https://arxiv.org/abs/2109.00580 .
4. Flaxman S, Mishra S, Gandy A, Unwin HJT, Mellan TA, Coupland H, et al. Estimating the Effects of NonPharmaceutical Interventions on COVID-19 in Europe. Nature. 2020;584(7820):257-61. Available from: https: //doi.org/10.1038/s41586-020-2405-7 .
5. Dehning J, Zierenberg J, Spitzner FP, Wibral M, Neto JP, Wilczek M, et al. Inferring Change Points in the Spread of COVID-19 Reveals the Effectiveness of Interventions. Science. 2020:eabb9789. Available from: https://www.sciencemag.org/lookup/doi/10.1126/science.abb9789 .
6. Alfano V, Ercolano S. The Efficacy of Lockdown Against COVID-19: A Cross-Country Panel Analysis. Applied Health Economics and Health Policy. 2020;18(4):509-17. Available from: http://link.springer.com/10.1007/ s40258-020-00596-3 .
7. Chu DK, Akl EA, Duda S, Solo K, Yaacoub S, Sch¨ unemann HJ, et al. Physical Distancing, Face Masks, and Eye Protection to Prevent Person-to-Person Transmission of SARS-CoV-2 and COVID-19: A Systematic Review and Meta-Analysis. The Lancet. 2020;395:1973-87. Available from: https://doi.org/10.1016/S0140-6736(20) 31142-9 .
8. Van Dyke ME PE Rogers TM, et al. Trends in County-Level COVID-19 Incidence in Counties With and Without a Mask Mandate - Kansas, June 1-August 23, 2020. Morbidity and Mortality Weekly Report. 2020;69(47):177781. Available from: https://www.cdc.gov/mmwr/volumes/69/wr/mm6947e2.htm .
9. Shakespeare T, Watson N, Brunner R, Cullingworth J, Hameed S, Scherer N, et al. Disabled People in Britain and the Impact of the COVID-19 Pandemic. Social Policy &amp; Administration. 2021. Available from: https: //onlinelibrary.wiley.com/doi/10.1111/spol.12758 .
10. Chodick G, Tene L, Patalon T, Gazit S, Ben Tov A, Cohen D, et al. Assessment of Effectiveness of 1 Dose of BNT162b2 Vaccine for SARS-CoV-2 Infection 13 to 24 Days After Immunization. JAMA Network Open. 2021 06;4(6):e2115985-5. Available from: https://doi.org/10.1001/jamanetworkopen.2021.15985 .
11. Thompson MG, Burgess JL, Naleway AL, Tyner H, Yoon SK, Meece J, et al. Prevention and Attenuation of Covid-19 with the BNT162b2 and mRNA-1273 Vaccines. New England Journal of Medicine. 2021;385(4):320-9. Available from: https://doi.org/10.1056/NEJMoa2107058 .
12. Griffin J, Haddix M, Danza P, et al. SARS-CoV-2 Infections and Hospitalizations Among Persons Aged ≥ 16 Years, by Vaccination Status - Los Angeles County, California, May 1-July 25, 2021. MMWR Mob Mortal Wkly Rep 2021. 2021;70:1170-6. Available from: https://www.cdc.gov/mmwr/volumes/70/wr/mm7034e5.htm? s\_cid=mm7034e5\_w#suggestedcitation .
13. Polack FP, Thomas SJ, Kitchin N, Absalon J, Gurtman A, Lockhart S, et al. Safety and Efficacy of the BNT162b2 mRNACOVID-19 Vaccine. New England Journal of Medicine. 2020;383(27):2603-15. PMID: 33301246. Available from: https://doi.org/10.1056/NEJMoa2034577 .
14. Baden LR, El Sahly HM, Essink B, Kotloff K, Frey S, Novak R, et al. Efficacy and Safety of the mRNA-1273 SARS-CoV-2 Vaccine. New England Journal of Medicine. 2021;384(5):403-16. PMID: 33378609. Available from: https://doi.org/10.1056/NEJMoa2035389 .

15. Sadoff J, Gray G, Vandebosch A, C´ ardenas V, Shukarev G, Grinsztejn B, et al. Safety and Efficacy of Single-Dose Ad26.COV2.S Vaccine against COVID-19. New England Journal of Medicine. 2021;384(23):2187-201. PMID: 33882225. Available from: https://doi.org/10.1056/NEJMoa2101544 .
16. Voysey M, Clemens SAC, Madhi S, Weckx LY, Folegatti PM, Aley PK, et al. Safety and Efficacy of the ChAdOx1 nCoV-19 Vaccine (AZD1222) Against SARS-CoV-2: An Interim Analysis of Four Randomised Controlled Trials in Brazil, South Africa, and the UK. The Lancet. 2021;397(10269):99-111. Available from: https://www. thelancet.com/journals/lancet/article/PIIS0140-6736(20)32661-1/fulltext#articleInformation .
17. Young C, Hall AM, Gon¸ calves-Bradley DC, Quinn TJ, Hooft L, van Munster BC, et al.. Home or Foster Home Care Versus Institutional Long-Term Care for Functionally Dependent Older People. Hoboken, NJ, USA: John Wiley and Sons, Ltd.; 2017. Available from: https://pubmed.ncbi.nlm.nih.gov/28368550/ .
18. Gorges RJ, Sanghavi P, Konetzka RT. A National Examination of Long-Term Care Setting, Outcomes, and Disparities Among Elderly Dual Eligibles. Health Affairs. 2019;38(7):1110-8. Available from: https://pubmed. ncbi.nlm.nih.gov/31260370/ .
19. He M, Li Y, Fang F. Is There a Link Between Nursing Home Reported Quality and COVID-19 Cases? Evidence from California Skilled Nursing Facilities. Journal of the American Medical Directors Association. 2020 Jul;21(7):905-8. Available from: https://pubmed.ncbi.nlm.nih.gov/32674817/ .
20. Kim JJ, Coffey KC, Morgan DJ, Roghmann MC. Lessons learned - Outbreaks of COVID-19 in Nursing Homes. Mosby Inc.; 2020. Available from: https://doi.org/10.1093/cid/ciz1045.
21. Abrams H, Loomer L, Gandhi A, Grabowski D. Characteristics of U.S. Nursing Homes with COVID-19 Cases. J Am Geriatr Soc. 2020;68:1653-6.
22. Li Y, Temkin-Greener H, Shan G, Cai X. COVID-19 Infections and Deaths among Connecticut Nursing Home Residents: Facility Correlates. Journal of the American Geriatrics Society. 2020;68(9):1899-906. Available from: https://pubmed.ncbi.nlm.nih.gov/32557542/ .
23. Chen MK, Chevalier JA, Long EF. Nursing Home Staff Networks and COVID-19. Proceedings of the National Academy of Sciences of the United States of America. 2021 Jan;118(1):e2015455118. Available from: http: //www.pnas.org/lookup/doi/10.1073/pnas.2015455118 .
24. Gorges RJ, Konetzka RT. Staffing Levels and COVID-19 Cases and Outbreaks in U.S. Nursing Homes. Journal of the American Geriatrics Society. 2020;68(11):2462-6. Available from: https://pubmed.ncbi.nlm.nih.gov/ 32770832/ .
25. CDC. Evidence Used to Update the List of Underlying Medical Conditions that Increase a Person's Risk of Severe Illness From COVID-19; 2020. Available from: https://www.cdc.gov/coronavirus/2019-ncov/ need-extra-precautions/evidence-table.html .
26. CDC. Disability and Health Data System; 2020. Availabe at https://dhds.cdc.gov/CR , accessed Oct 2020. Available from: https://dhds.cdc.gov/CR .
27. PHI. Direct Care Workers in the United States; 2020. Available from: https://phinational.org/resource/ direct-care-workers-in-the-united-states-key-facts/ .
28. Adhikari, Samrachana; Pantaleo, Nicholas P ; Feldman, Justin M ; Ogedegbe, Olugbenga; Thorpe, Lorna; Troxel AB. Assessment of Community-Level Disparities in Coronavirus Disease 2019 (COVID-19) Infections and Deaths in Large US Metropolitan Areas. JAMA Network Open. 2020;3(7). Available from: https://jamanetwork. com/journals/jamanetworkopen/fullarticle/2768723?resultClick=1 .
29. Egede LE, Walker RJ. Structural Racism, Social Risk Factors, and COVID-19 - A Dangerous Convergence for Black Americans. New England Journal of Medicine. 2020;383(e77). Available from: https://www.nejm.org/ doi/full/10.1056/NEJMp2023616 .

30. Kim SJ, Bostwick W. Social Vulnerability and Racial Inequality in COVID-19 Deaths in Chicago. Health Education and Behavior. 2020;47(4):509-13. Available from: https://journals.sagepub.com/doi/10.1177/ 1090198120929677 .
31. Okoro CA, Hollis ND, Cyrus AC, Griffin-Blake S. Prevalence of Disabilities and Health Care Access by Disability Status and Type Among Adults - United States, 2016; 2018. Available from: https://www.cdc.gov/mmwr/ volumes/67/wr/mm6732a3.htm?s{\_}cid=mm6732a3{\_}w .
32. Lauer E, Houtenville A. 2017 Annual Disability Statistics Supplement. Institute on Disability, University of New Hampshire. 2018. Available from: https://disabilitycompendium.org/sites/default/files/ user-uploads/2017\_AnnualReport\_FINAL.pdf .
33. NJ COVID-19 Disability Action Committee. NJ COVID-19 Disability Action Committee: Initial Report; 2020. Available from: https://www.adacil.org/latest-updates/covid-19-disability-report .
34. Ritchie H, Mathieu E, Rod´ es-Guirao L, Appel C, Giattino C, Ortiz-Ospina E, et al. Coronavirus Pandemic (COVID-19). Our World in Data. 2020. Https://ourworldindata.org/coronavirus.
35. Lindstrom MR, Porter MA, Shoenhard H, Trick S, Valles TE, Zinski JM. Networks of Necessity: Preventing COVID-19 Among Disabled People and Their Caregivers; 2020. ftp://ftp.math.ucla.edu/pub/camreport/ cam20-33.pdf .
36. Estrada E. COVID-19 and SARS-CoV-2. Modeling the present, looking at the future. Physics Reports. 2020;869:1-51.
37. Arino J. Describing, modelling and forecasting the spatial and temporal spread of COVID-19 - A short review. 2021. arXiv:2102.02457. Available from: https://arxiv.org/abs/2102.02457 .
38. Zaplotnik, ˇ Ziga and Gavri´ c, Aleksandar and Medic, Luka. Simulation of the COVID-19 Epidemic on the Social Network of Slovenia: Estimating the Intrinsic Forecast Uncertainty. PLoS ONE. 2020;15(8). Available from: https://doi.org/10.1371/journal.pone.0238090 .
39. Sameni R. Mathematical Modeling of Epidemic Diseases; A Case Study of the COVID-19 Coronavirus. arXiv preprint arXiv:200311371. 2020. Available from: https://arxiv.org/abs/2003.11371 .
40. Browning R, Sulem D, Mengersen K, Rivoirard V, Rousseau J. Simple Discrete-Time Self-Exciting Models Can Describe Complex Dynamic Processes: A Case Study of COVID-19. medRxiv. 2020. Available from: https://www.medrxiv.org/content/early/2020/11/03/2020.10.28.20221077 .
41. Escobar JV. A Hawkes Process Model for the Propagation of COVID-19: Simple Analytical Results. EPL (Europhysics Letters). 2020;131(6):68005. Available from: https://doi.org/10.1209/0295-5075/131/68005 .
42. Bertozzi AL, Franco E, Mohler G, Short MB, Sledge D. The challenges of modeling and forecasting the spread of COVID-19. Proceedings of the National Academy of Sciences. 2020;117(29):16732-8. Available from: https: //www.pnas.org/content/117/29/16732 .
43. Hoertel N, Blachier M, Blanco C, et al. A Stochastic Agent-Based Model of the SARS-CoV-2 Epidemic in France. Nature Medicine. 2020;26:1417-21. Available from: https://doi.org/10.1038/s41591-020-1001-6 .
44. Arenas A, Cota W, G´ omez-Garde˜ nes J, G´ omez S, Granell C, Matamalas JT, et al. Modeling the Spatiotemporal Epidemic Spreading of COVID-19 and the Impact of Mobility and Social Distancing Interventions. Physical Review X. 2020;10:041055. Available from: https://link.aps.org/doi/10.1103/PhysRevX.10.041055 .
45. Pullano G, Valdano E, Scarpa N, Rubrichi S, Colizza V. Evaluating the Effect of Demographic Factors, Socioeconomic Factors, and Risk Aversion on Mobility During the COVID-19 Epidemic in France Under Lockdown: A Population-Based Study. The Lancet. 2020;2(12):e638-49. Available from: https://doi.org/10.1016/ S2589-7500(20)30243-0 .

46. Kraemer MUG, Yang CH, Gutierrez B, Wu CH, Klein B, Pigott DM, et al. The Effect of Human Mobility and Control Measures on the COVID-19 Epidemic in China. Science. 2020;368(6490):493-7. Available from: https://science.sciencemag.org/content/368/6490/493 .
47. Bertozzi AL, Franco E, Mohler G, Short MB, Sledge D. The challenges of modeling and forecasting the spread of COVID-19. Proceedings of the National Academy of Sciences of the United States of America. 2020;117(29):16732-8. Available from: https://www.pnas.org/content/117/29/16732 .
48. Lai S, Bogoch II, Ruktanonchai NW, Watts A, Lu X, Yang W, et al. Assessing Spread Risk of Wuhan Novel Coronavirus Within and Beyond China, January-April 2020: A Travel Network-Based Modelling Study; 2020. Available from: https://www.ncbi.nlm.nih.gov/pmc/articles/PMC7276059/ .
49. Holme P, Saram¨ aki J. Temporal Networks. Physics Reports. 2012;519(3):97-125. Available from: https: //arxiv.org/abs/1108.1780 .
50. Dakiche N, Tayeb FBS, Slimani Y, Benatchba K. Tracking Community Evolution in Social Networks: A Survey. Information Processing &amp; Management. 2019;56(3):1084-102. Available from: https://doi.org/10.1016/j. ipm.2018.03.005 .
51. Melnik S, Porter MA, Mucha PJ, Gleeson JP. Dynamics on Modular Networks with Heterogeneous Correlations. Chaos: An Interdisciplinary Journal of Nonlinear Science. 2014;24(2):023106. Available from: https://arxiv. org/abs/1207.1809 .
52. Miller JC, Volz EM. Incorporating Disease and Population Structure Into Models of SIR Disease in Contact Networks. PloS One. 2013;8(8):e69162. Available from: https://doi.org/10.1371/journal.pone.0069162 .
53. Brauer F, Castillo-Chavez C, Feng Z. Mathematical Models in Epidemiology. Heidelberg, Germany: SpringerVerlag; 2019.
54. Kiss IZ, Miller JC, Simon PL. Mathematics of Epidemics on Networks: From Exact to Approximate Models. Cham, Switzerland: Springer International Publishing; 2017.
55. Pastor-Satorras R, Castellano C, Van Mieghem P, Vespignani A. Epidemic Processes in Complex Networks. Reviews of Modern Physics. 2015;87(3):925. Available from: https://journals.aps.org/rmp/abstract/10. 1103/RevModPhys.87.925 .
56. Government of Tasmania. COVID-19 North West Regional Hospital Outbreak Interim Report; 2020. https://www.health.tas.gov.au/\_\_data/assets/pdf\_file/0006/401010/North\_West\_Regional\_ Hospital\_Outbreak\_-\_Interim\_Report.pdf .
57. Ottawa Public Health. Daily COVID-19 Dashboard. Ottawa Public Health; 2020. Accessed: 2020-08-24. https: //www.ottawapublichealth.ca/en/reports-research-and-statistics/daily-covid19-dashboard.aspx .
58. Neilsen K. A Timeline of the Novel Coronavirus in Ontario. Global News; 2020. Accessed: 2020-08-24. https: //globalnews.ca/news/6859636/ontario-coronavirus-timeline/ .
59. City of Ottawa. City of Ottawa's Reopening Plan; 2020. https://documents.ottawa.ca/sites/documents/ files/reopeningbooklet\_en.pdf .
60. Baek EC, Porter MA, Parkinson C. Social network analysis for social neuroscientists. Social Cognitive and Affective Neuroscience. 2020 05;16(8):883-901. Available from: https://doi.org/10.1093/scan/nsaa069 .
61. Fosdick BK, Larremore DB, Nishimura J, Ugander J. Configuring Random Graph Models with Fixed Degree Sequences. SIAM Review. 2018;60(2):315-55.
62. Newman MEJ. Networks. 2nd ed. Oxford, UK: Oxford University Press; 2018.
63. Statistics Canada. Census Profile, 2016 Census; 2019. Accessed: 2020-06-21. https://www12.statcan.gc.ca/ census-recensement/2016/dp-pd/prof/details/page.cfm?Lang=E .

64. U S Bureau of Labor Statistics. Employment Projections; 2019. Accessed: 2020-10-31. https://www.bls.gov/ emp/tables/emp-by-detailed-occupation.htm .
65. McNicholas C, Poydock M. Who are Essential Workers?; 2020. Accessed: 2020-10-31. https://www.epi.org/ blog/who-are-essential-workers-a-comprehensive-look-at-their-wages-demographics-and-unionization-rat
66. United States Census Bureau. National Population Totals and Components of Change: 2010-2019; 2020. Accessed: 2020-10-31. https://www.census.gov/data/tables/time-series/demo/popest/ 2010s-national-total.html .
67. Altman D, Kaiser Family Foundation. Most Americans are Practicing Social Distancing; 2020. Accessed: 2020-06-21. https://www.axios.com/ coronavirus-social-distancing-lockdown-polling-7c27d86f-bb4b-4cbf-aedf-cfdd26799fd1.html .
68. Tian T, Huo X. Secondary attack rates of COVID-19 in diverse contact settings, a meta-analysis. The Journal of Infection in Developing Countries. 2020;14(12):1361-7.
69. Madewell ZJ, Yang Y, Longini IM, Halloran ME, Dean NE. Household transmission of SARS-CoV-2: A systematic review and meta-analysis. JAMA Network Open. 2020;3(12):e2031756.
70. Anderson RM, Heesterbeek H, Klinkenberg D, Hollingsworth TD. How will country-based mitigation measures influence the course of the COVID-19 epidemic? The Lancet. 2020;395(10228):931-4.
71. Buitrago-Garcia D, Egli-Gany D, Counotte MJ, Hossmann S, Imeri H, Ipekci AM, et al. Occurrence and Transmission Potential of Asymptomatic and Presymptomatic SARS-CoV-2 Infections: A Living Systematic Review and Meta-Analysis. PLoS Medicine. 2020;17(9):e1003346. Available from: https://doi.org/10.1371/ journal.pmed.1003346 .
72. Byrne AW, McEvoy D, Collins AB, Hunt K, Casey M, Barber A, et al. Inferred duration of infectious period of SARS-CoV-2: Rapid scoping review and analysis of available evidence for asymptomatic and symptomatic COVID-19 cases. BMJ Open. 2020;10(8):e039856.
73. Ma S, Zhang J, Zeng M, Yun Q, Guo W, Zheng Y, et al. Epidemiological parameters of coronavirus disease 2019: A pooled analysis of publicly reported individual data of 1155 cases from seven countries. MedRxiv. 2020. Available from: doi:https://doi.org/10.1101/2020.03.21.20040329 .
74. Hu Z, Song C, Xu C, Jin G, Chen Y, Xu X, et al. Clinical characteristics of 24 asymptomatic infections with COVID-19 screened among close contacts in Nanjing, China. Science China Life Sciences. 2020;63:706-11. Available from: https://doi.org/10.1007/s11427-020-1661-4 .
75. Bajema KL, Oster AM, McGovern OL, Lindstrom S, Stenger MR, Anderson TC, et al. Persons Evaluated for 2019 Novel Coronavirus- United States, January 2020. Morbidity and Mortality Weekly Report. 2020;69(6):16670. Available from: https://www.cdc.gov/mmwr/volumes/69/wr/mm6906e1.htm .
76. Jiehao C, Jin X, Daojiong L, Zhi Y, Lei X, Zhenghai Q, et al. A case series of children with 2019 novel coronavirus infection: Clinical and epidemiological features. Clinical Infectious Diseases. 2020;71(6):1547-51.
77. Faes C, Abrams S, Van Beckhoven D, Meyfroidt G, Vlieghe E, Hens N, et al. Time between symptom onset, hospitalisation and recovery or death: Statistical analysis of Belgian COVID-19 patients. International Journal of Environmental Research and Public Health. 2020;17(20):7560.
78. Guan Wj, Ni Zy, Hu Y, Liang Wh, Ou Cq, He Jx, et al. Clinical Characteristics of Coronavirus Disease 2019 in China. New England Journal of Medicine. 2020;382(18):1708-20. Available from: https://www.nejm.org/doi/ full/10.1056/NEJMoa2002032 .
79. Rothwell J. Americans' Social Contacts During the COVID-19 Pandemic. Gallup Blog; 2020. Accessed: 2020-10-31. https://news.gallup.com/opinion/gallup/308444/ americans-social-contacts-during-covid-pandemic.aspx .

80. UNdata. UN City population; 2017. Accessed: 2020-08-24. https://www.google.com/publicdata/explore? ds=z5567oe244g0ot\_ .
81. Pullano G, Di Domenico L, Sabbatini CE, Valdano E, Turbelin C, Debin M, et al. Underdetection of cases of COVID-19 in France threatens epidemic control. Nature. 2021;590(7844):134-9.
82. Wang Y, Chakrabarti D, Wang C, Faloutsos C. Epidemic Spreading in Real Networks: An Eigenvalue Viewpoint. In: Proceedings of the 22nd International Symposium on Reliable Distributed Systems; 2003. p. 25-34. Available from: https://ieeexplore.ieee.org/document/1238052 .
83. Gozzi N, Scudeler M, Paolotti D, Baronchelli A, Perra N. Self-Initiated Behavioral Change and Disease Resurgence on Activity-Driven Networks. Physical Review E. 2021;104:014307. Available from: https: //link.aps.org/doi/10.1103/PhysRevE.104.014307 .
84. Kuper H, Lena Morgon B, Bright T, et al. Disability-Inclusive COVID-19 Response: What It Is, Why It Is Important and What We Can Learn From the United Kingdom's Response [version 1; peer review: 2 approved]. Wellcome Open Research. 2021;5:79. Available from: https://wellcomeopenresearch.org/articles/5-79/ v1 .
85. Anderez DO, Kanjo E, Pogrebna G, Johnson S, Hunt JA. A Modified Epidemiological Model to Understand the Uneven Impact of COVID-19 on Vulnerable Individuals and the Approaches Required to Help them Emerge from Lockdown; 2020. Available from: http://arxiv.org/abs/2006.10495 .
86. Bahl R, Eikmeier N, Fraser A, Junge M, Keesing F, Nakahata K, et al. Modeling COVID-19 Spread in Small Colleges. PLoS ONE. 2021;16:e0255654. Available from: https://journals.plos.org/plosone/article?id= 10.1371/journal.pone.0255654 .
87. Li KKF, Jarvis SA, Minhas F. Elementary Effects Analysis of Factors Controlling COVID-19 Infections in Computational Simulation Reveals the Importance of Social Distancing and Mask Usage. Computers in Biology and Medicine. 2021;134:104369. Available from: https://www.sciencedirect.com/science/article/pii/ S0010482521001633 .
88. Tian Y, Sridhar A, Yagan O, Poor HV. Analysis of the Impact of Mask-wearing in Viral Spread: Implications for COVID-19; 2020. Available from: http://arxiv.org/abs/2011.04208 .
89. Wang Z, Bauch CT, Bhattacharyya S, d'Onofrio A, Manfredi P, Perc M, et al. Statistical Physics of Vaccination. Physics Reports. 2016;664:1 113. Available from: http://www.sciencedirect.com/science/article/pii/ S0370157316303349 .
90. CDC National Center for Immunization and Respiratory Diseases. Different COVID-19 Vaccines; 2020. Available from: https://www.cdc.gov/coronavirus/2019-ncov/vaccines/different-vaccines.html .
91. European Medicines Agency. EMA Recommends First COVID-19 Vaccine for Authorisation in the EU; 2020. Available from: https://www.ema.europa.eu/en/news/ ema-recommends-first-covid-19-vaccine-authorisation-eu .
92. Health Canada. Drug and Vaccine Authorizations for COVID-19: List of Authorized Drugs, Vaccines and Expanded Indications - Canada.ca; 2020. Available from: https://www.canada.ca/en/health-canada/ services/drugs-health-products/covid19-industry/drugs-vaccines-treatments/authorization/ list-drugs.html .
93. Medicines and Healthcare products Regulatory Agency. Oxford University/AstraZeneca COVID19 vaccine approved; 2020. Available from: https://www.gov.uk/government/news/ oxford-universityastrazeneca-covid-19-vaccine-approved .
94. Brown CM. Outbreak of SARS-CoV-2 Infections, Including COVID-19 Vaccine Breakthrough Infections, Associated with Large Public Gatherings - Barnstable County, Massachusetts, July 2021. MMWR Morbidity and Mortality Weekly Report. 2021;70.

95. Kates J, Dawson L, Anderson E, Rouw A, Michaud J, Singer N. COVID-19 Vaccine Breakthrough Cases: Data from the States; 2021. Accessed: 2021-08-13. https://www.kff.org/policy-watch/ covid-19-vaccine-breakthrough-cases-data-from-the-states/ .
96. Chia PY, Xiang Ong SW, Chiew CJ, Ang LW, Chavatte JM, Mak TM, et al. Virological and Serological Kinetics of SARS-CoV-2 Delta Variant Vaccine-Breakthrough Infections: A Multi-Center Cohort study. medRxiv. 2021. Available from: https://www.medrxiv.org/content/early/2021/07/31/2021.07.28.21261295 .
97. de Gier B, Andeweg S, Joosten R, ter Schegget R, Smorenburg N, van de Kassteele J, et al. Vaccine effectiveness against SARS-CoV-2 transmission and infections among household and other close contacts of confirmed cases, the Netherlands, February to May 2021. Eurosurveillance. 2021;26(31). Available from: https://doi.org/10. 2807/1560-7917.ES.2021.26.31.2100640 .
98. Toner E, Barnill A, Krubiner C, Bernstein J, Privor-Dumm L, Watson M, et al. Interim Framework for COVID19 Vaccine Allocation and Distribution in the United States. The Johns Hopkins Center for Health Security; 2020. Available from: https://www.centerforhealthsecurity.org/our-work/pubs\_archive/pubs-pdfs/ 2020/200819-vaccine-allocation.pdf .
99. Bedson J, Skrip LA, Pedi D, Abramowitz S, Carter S, Jalloh MF, et al. A review and agenda for integrated disease models including social and behavioural factors. Nature Human Behaviour. 2021. https://doi.org/ 10.1038/s41562-021-01136-2 .
100. Brainard J, Jones N, Lake I, Hooper L, Hunter PR. Facemasks and similar barriers to prevent respiratory illness such as COVID-19: A rapid systematic review. medRxiv. 2020. Available from: https://www.medrxiv.org/ content/early/2020/04/06/2020.04.01.20049528 .
101. Mossong J, Hens N, Jit M, Beutels P, Auranen K, Mikolajczyk R, et al. Social Contacts and Mixing Patterns Relevant to the Spread of Infectious Diseases. PLoS Medicine. 2008;5(3):e74. Available from: https://doi. org/10.1371/journal.pmed.0050074 .
102. Rothwell J. A Survey of Essential Workers Shows a Political Divide. The New York Times. 2020 Apr. https: //www.nytimes.com/2020/04/27/upshot/red-blue-workplace-differences-coronavirus.html .

## Supporting Information

## A Estimates of Parameter Values

We present the assumptions and derivations that we use to estimate the parameters in our model (see Section 2). These include parameters that we can obtain directly (possibly with some inference) from the literature and ones that we fit from case data in Ottawa. In this section, we use log to denote the natural logarithm.

## A.1 Parameters that we Infer from the Literature

## A.1.1 Properties of Exponential Distributions

Because we assume that transition times between disease states come from exponential distributions, we state a few useful properties of exponential random variables.

For a random variable X that one samples from an exponential distribution with rate λ (i.e., X ∼ Exp( λ )), the probability density function is f ( x ) = λ e -λx , the mean is 1 /λ , and the median is log 2 /λ .

Suppose that we have a random variable Y = min { Y 1 , Y 2 } , where Y 1 and Y 2 are random variables that we sample from exponential distributions of rates λ 1 and λ 2 , respectively. It then follows that Y is an exponential random variable with rate λ 1 + λ 2 and the probability that Y 1 &lt; Y 2 is λ 1 / ( λ 1 + λ 2 ) .

## A.1.2 Parameters

Transition Rate from Exposed to Asymptomatic ( ν ). The rate of moving from the exposed compartment to a contagious state (and hence to the asymptomatic compartment in our model) has been estimated at ν = 1 day -1 [70].

Recovery Rate from Hospitalization ( ζ ). The mean duration of hospitalization has been estimated to be 1 /ζ = 12 . 8 days [78], so ζ ≈ 0 . 0781 day -1 .

Transition Rates from Asymptomatic to Ill ( α ) and Recovered ( η ). It has been estimated that 19 . 45% of cases are entirely asymptomatic [71], so η η + α = 0 . 1945. Byrne et al. [72] summarized many relevant studies that give data about different transition rates. From these studies, we note that the mean duration in the asymptomatic state has been estimated to be about 1 α + η = 7 . 25 days [73] and the median duration has been estimated to be about log 2 α + η = 9 . 5 days [74]. We take the mean of these two values to estimate 1 η + α ≈ 10 . 478 days, which we combine with η η + α = 0 . 1945 to obtain α ≈ 0 . 07688 day -1 and η ≈ 0 . 01856 day -1 .

Transition Rates from Ill to Hospitalized ( µ ) and Recovered ( ρ ). It has been estimated that approximately µ µ + ρ = 20% of the symptomatic cases of COVID-19 result in hospitalization [75]. Among children with mild cases of COVID-19, the median duration from the onset of symptoms onset no longer being infectious is about log 2 µ + ρ = 12 days [76]. (This study was also referenced in Byrne et al. [72].) In Belgium, the median duration from the onset of symptoms to hospitalization was estimated to be log 2 µ + ρ = 5 days [77]. We take the mean of the values from these two studies and estimate 1 µ + ρ ≈ 12 . 2629. With µ µ + ρ = 0 . 2, we obtain µ ≈ 0 . 01631 day -1 and ρ ≈ 0 . 06524 day -1 .

Mask Risk-Reduction Factor ( m ). Based on three different viruses (SARS CoV-2, SARS-CoV, and MERS-CoV), an unadjusted relative risk when wearing a face mask versus not wearing one and contracting an infection has been reported to be 0 . 34 (with a 95% confidence window of 0 . 26 to 0 . 45) [7]. These results include both healthcare settings and non-healthcare settings. Because the three viruses are from the same family, it was argued in [7] that their relative risks should be comparable. For the data that they reported, it is not clear if only one or both individuals wore masks in their interactions. We use m = 0 . 34 to represent the risk reduction when both individuals in an interaction wear masks, and we use √ m ≈ 0 . 5831 if only one individual in an interaction wears a mask. That is, if only one individual in an interaction wears a mask, we quantify the transmission risk as the geometric mean of the best-case transmission reduction if both individuals wear a mask and the worst-case transmission reduction if neither individual wears a mask. By definition, given values q 1 , q 2 , . . . , q n , their geometric mean is ( q 1 × q 2 × · · · × q n ) 1 /n . Although our choice seems arbitrary, according to [100], there is a small reduction in the chance of becoming infected in people who wear masks within a household, so it seems plausible that one individual wearing a mask in an interaction between two people confers some reduction in transmission.

Probability of Breaking Weak Contacts if Symptomatic ( b ). It was very difficult to estimate this parameter. Ultimately, we use the fact that 92% of people in a survey reported practicing physical distancing [67] as a proxy for the portion of a population who would break their weak contacts if they became symptomatic. That is, b = 0 . 92.

Baseline Transmission Probability β and Caregiving ( w c ) and Weak ( w w ) Edge Weights. We estimate β and these edge weights based on reported secondary attack rates in various scenarios. The secondary attack rate describes the fraction of a contagious individual's contacts who become infected as a result of interacting with that individual. The secondary attack rate among weak contacts [68] appears to range from about 1% to about 6%, so we estimate it to be 3 . 5%. Additionally, the secondary attack rate within a household has been estimated to be approximately 20% [68] and is much higher (about 37 . 8%) between spouses [69].

Caregiving work is extremely intimate and requires extended, close physical contact and potential exposure to bodily fluids. Such a level of intimacy is not typical between housemates, so we use the secondary attack rate between spouses as a proxy for the level of risk in an interaction between a caregiver and a disabled person.

We conduct a set of simulations to estimate the secondary attack rate for each type of contact. The secondary attack rate is the fraction of a contagious individual's contacts that they infect on average. In each trial, we assign a contagious duration D c (asymptomatic time plus possibly symptomatic time, depending on contact type and whether contacts are broken if an individual becomes ill) and compute the probability that that contagious individual infects somebody. For weak contacts 1 , we use a daily transmission probability of √ mw w β ; for strong contacts, we use a probability of β ; for

1 We obtain the value √ m by estimating the risk mitigation of masks as the geometric mean of the value (1) when no individual in an interaction wears a mask and the value ( m ) when both individuals in an interaction wear a mask. We use the geometric mean because of the uncertainty in whether or not people wear masks.

caregiving contacts, we use a probability of w c β .

Therefore, in a single trial, the probability of infection via a strong contact is 1 -(1 -β ) D c . We then determine the values of w w , β , and w c so that when averaged over many trials, the mean probability of passing on COVID-19 matches the above secondary attack rates. This yields β ≈ 0 . 0112, w w ≈ 0 . 473, and w c ≈ 2 . 268.

Subpopulation Proportions of the Total Population. By combining the fraction of the population that has a cognitive disability with the fraction that has a physical disability that causes difficulty in dressing, bathing, or getting around inside the home, we estimate that the fraction of our population who are disabled and receive assistance from professional caregivers is f dis ≈ 0 . 073 [32]. Unfortunately, there is a paucity of readily available data, so this is a rough estimate. From the United States Bureau of Labor Statistics, a fraction f care ≈ 0 . 021 of the U.S. population is employed as a home health/professional care aid [64]. We use this number as an estimate of the proportion of the population that provides care. This is likely an underestimate because many people provide care in unpaid settings. From an estimated 55,217,845 essential workers in the United States [65], whose population in July 2019 was estimated to be 328,239,523 [66], the fraction of essential workers is approximately 0 . 1682. After subtracting the people who are caregivers, we obtain that a fraction f ess ≈ 0 . 1472 of the population are essential workers. That leaves the fraction f gen ≈ 0 . 7588 for the remaining population (i.e., the general population).

Mean Numbers of Contacts. We need distributions of the numbers of family contacts, weak contacts (through work, shopping, seeing friends, and so on), and caregiving contacts. We begin by focusing on the mean values and later consider the distributions themselves. From the 2016 Canadian census [63], households have a mean value of 2 . 4 members, which implies that individuals have a mean of ¯ F = 1 . 4 strong contacts.

From Gallup data in April 2020 [79], during pandemic lockdowns, the people who were surveyed had a mean of 5 . 1 contacts per day at work and a mean of 4 contacts per day outside of work and home. Additionally, 27% of working adults completely isolated themselves except to members of their own household. In Europe in 2008, the overall population had a mean of 13 . 4 daily contacts without a lockdown in place [101]. In April 2020, essential workers saw a mean of 22 contacts per day (a much larger number than people who are not essential workers) during the lockdown [102]. By combining these disparate pieces of data, we are able to make some relevant estimates.

Let O gd denote the mean number of occupational contacts of the general and disabled subpopulations each day without a lockdown, O c denote the mean number of disabled people that a caregiver sees in a day, O ∗ gd denote the mean number of occupational contacts of the general and disabled subpopulations each day during a lockdown, w denote the mean number of weak contacts (outside of work) of the overall population each day without a lockdown, w ∗ denote the mean number of weak contacts (outside of work) of the overall population each day with a lockdown, and O e denote the mean number of occupational contacts of essential workers each day (both with and without a lockdown). Our parenthetical comment about O e indicates that we are assuming that the number of work contacts is the same for essential workers regardless of whether or not there is a lockdown. We also assume that w does not depend on an individual's subpopulation (disabled person, caregiver, essential worker, or member of the general population). Likewise, we assume that w ∗ does not depend on an individual's subpopulation.

From the data that we cited two paragraphs ago, we estimate that w ∗ = 4 and each disabled person sees 2 caregivers per day. Additionally, O c = 2 f dis f care ≈ 6 . 95 and

<!-- formula-not-decoded -->

To close the system of equations and obtain our estimates, we require one further assumption. If 27% of workers isolate at home, then the mean number of contacts at work is

<!-- formula-not-decoded -->

We obtain w ≈ 5 . 14, w ∗ ≈ 4, O e ≈ 16 . 23, O c ≈ 6 . 95, O gd ≈ 5 . 20, and O ∗ gd ≈ 3 . 08. When we use approximate truncated power-law distributions to model the possibility that some people have many contacts and others have few contacts, we want to satisfy the following criteria:

- the general population has a mean of w + O gd ≈ 10 . 34 weak contacts per day when not physically distancing and a mean of w ∗ + O ∗ gd ≈ 7 . 08 weak contacts per day when physically distancing;
- the disabled subpopulation has the same mean value of weak contacts as the general population whether or not people are physically distancing;
- the caregiver subpopulation has a mean of w ≈ 5 . 14 weak contacts per day when not physically distancing and a mean of w ≈ 4 weak contacts per day when physically distancing; and
- the essential-worker subpopulation has a mean of O e + w ≈ 21 . 37 weak contacts per day when not physically distancing and a mean of O e + w ∗ ≈ 20 . 23 contacts per day when physically distancing.

Although the caregiver subpopulation may seem to have very few weak contacts, we note that most of their daily contacts come from O c , which we estimate separately from the ordinary weak contacts.

Although the number of weak contacts for essential workers does decrease slightly during a lockdown, we use O e + w whether or not a lockdown is in place as an approximation because the difference in the numbers of weak contacts is very small (21 . 37 versus 20 . 23). In practice, it was difficult for us to reduce the mean number of contacts slightly in this situation, because picking the minimum of two random variables of similar distributions tends to result in a value that is much smaller than the original one and doing so would result in the essential workers having far too few contacts.

Distribution of Strong Contacts: We use data from the 2016 Canadian census [63] to describe the distribution of household sizes. According to these data, 105,750 households consist of 1 person, 124,280 households consist of 2 people, 58,010 households consist of 3 people, 55,215 households consist of 4 people, and 30,500 households consist of 5 or more people (which we treated as exactly 5 people). From these data, we construct an empirical distribution that we use for the entire population. It is D s = E (0 . 283 , 0 . 332 , 0 . 155 , 0 . 148 , 0 . 0816).

Caregivers: To each disabled person, we assign one strong caregiver and one weak caregiver with whom they interact each day (although they do not interact with the latter when either they or the caregiver is symptomatic). We chose the weak caregivers from a pool of caregivers. We use 10 as the baseline caregiver-pool size, but we also consider other sizes (4 and 25, as we discuss in Section 3).

## A.2 Fits from Data

There are three other parameters in our model that we also need to estimate. Even with our many estimates from the literature that we discussed in Section A.1, we still need to estimate the following quantities: (1) the maximum number C ∗ of weak contacts that an individual has; (2) the number A 0 of people who are asymptomatic on day 0; and (3) the probability τ that an individual who is symptomatically ill but not hospitalized is counted in the cumulative number of cases.

We model the number of weak contacts using an approximate truncated power-law distribution. That is, the daily number of weak contacts of an individual is distributed according to P (0 , C ∗ ; O q ), where O q denotes the mean number of weak contacts of subpopulation q .

Based on the simulation procedure that we described in Section B, we use a fitting procedure (along with case data from Ottawa [57]) to estimate τ and C ∗ with a grid search. We use the first 90 days as fitting data and assume that the associated contact distributions and mask-wearing policies are instantly adopted on day 44 (i.e., the start of the lockdown in Ottawa). We tried fitting over shorter time windows, but these yielded poorer fits. The likely reason for the poor fits for these shorter time windows is that the parameter C ∗ is smaller when fit over shorter time intervals (because the disease has spread less at that stage). The longer time window allows C ∗ , which may be a key driver in the disease dynamics, to be fit to a larger value and thereby allow extensive spreading of the disease.

We assume that there are A 0 people on day 0 in the asymptomatic compartment and that all other individuals are in the susceptible compartment. On day 1, with the first recorded case, there is 1 recorded case in expectation. Therefore,

<!-- formula-not-decoded -->

The first factor is the probability that the transition from the A compartment to the I compartment occurs before the transition from A to the R compartment. The second factor is the probability there is a transition out of the A compartment in a 1-day time period. The third factor is the probability that an individual in the I compartment tests positive for COVID-19. The fourth factor ( A 0 ) is the total number of asymptomatic people on day 0. Our choice to make the expected number of documented cases equal to 1 on day 1 allows us to have two parameters (rather than three) when fitting. Using more parameters can result in overfitting.

We seek to minimize the glyph[lscript] 2 -error in new daily cases (i.e., the change in the daily cumulative case count). Because our stochastic model is complicated, with variation across trials, we use a grid search (instead of a gradient-based method) to estimate parameters. In Table 2, we summarize our results. From this procedure, our 'optimal' parameter values are τ = 0 . 04 and C ∗ = 60.

## B Simulations of our Stochastic Model of COVID-19 Spread

## B.1 Simulation Steps

We summarize our simulation procedure in Algorithm 1. Note that it uses the other algorithms that we present in this subsection. The code is available at our Bitbucket repository.

We initially construct a network by matching ends of edges (i.e., 'stubs') in a generalization of a configurationmodel network. For weak contacts, we assign a number of stubs to each individual in each subpopulation to encode their number of weak contacts (see Algorithm 2). We determine this number from an associated probability distribution. We then do a so-called 'random matching' (see Algorithm 4), in which we match stubs uniformly at random. Pairs of individuals whose stubs are matched are contacts of each other. If we choose two individuals who are already contacts or an individual is paired with themself, we simply discard that pairing. For strong contacts, we assign individuals to units (see Algorithm 3) and make members of these units strong contacts with each other unless they are already contacts (see Algorithm 5). Consequently, the number of contacts per individual does not perfectly match the desired distributions. However, for a network with many nodes, these errors are negligible in practice. See [61] for a detailed exposition of different types of configuration models (although we employ a generalization of a configuration model), including different strategies for how to deal with self-edges and multi-edges. We assign weak and strong caregivers to disabled people in a manner that is analogous to how we assign strong contacts (see Algorithm 6).

We then place some number of individuals, who we choose uniformly at random from the nodes in the network, into the A and/or I compartments. This number of individuals, which subpopulation they belong, and the choice of these compartments (all of these individuals in A, all of these individuals in I, or some of these individuals in A and some of them in I) depends on user input. For example, in the four simulations that we used to generate Figure 10, all initially infected individuals are in the A compartment and belong to the general subpopulation, caregiver subpopulation, disabled subpopulation, and essential-worker subpopulation, respectively. In all other simulations that we discuss in the present paper, the initially infected individuals are all in the A compartment. After having initialized the contact structures, we execute the commands in the following paragraphs for a user-specified number of iterations.

We check if we need to update contact structures and/or mask-wearing strategies because of a lockdown (see Algorithm 9) or a reopening (see Algorithm 10). For a lockdown, we update the mask-wearing strategies and assign each individual a number of weak contacts from the new weak-contact distributions. If the new number of weak contacts is less than the current number of weak contacts, we remove excess contacts uniformly at random. For a reopening, we again update the mask-wearing strategies and assign each individual a number of weak contacts from the new weakcontact distribution. If the new number of weak contacts is larger than the current number of weak contacts, we assign the individual a number of stubs that is equal to the difference between the new sample and the current number of contacts and apply Algorithm 4 to connect the stubs.

On each day, we assign a weak caregiver to each disabled person uniformly at random from their pool of weak caregivers, as long as neither is breaking their contacts. We then use Algorithm 8 to determine if each individual in the network remains in their current compartment or moves to a new one. If an individual is in the S compartment, we calculate the probability of infection using Algorithm 7. In this algorithm, we loop through each of this individual's contagious contacts (i.e., those in the A, I, or H compartments) and use Equation 1 to calculate the probability that the individual becomes infected. For compartments E and H, for which there is only one possible transition to a new compartment, we draw a transition time from Exp( χ ) (where χ is the associated rate constant) to determine if there is a transition between compartments. If the time is less than 1 day, then the individual changes compartments; otherwise,

Table 2. The glyph[lscript] 2 -error in new daily documented cases for various values of τ and C ∗ . Using (3), with values of α and µ from the literature and a given value of τ , we determine A 0 . For each set of parameters, we conduct 96 trials and we compute the error by taking the mean of all trials in which there are at least 250 documented cases through day 90. We only report parameter values for which the errors are smaller than 5 × 10 4 . We test all parameter values on the lattice ( τ, C ∗ ) ∈ { 0 . 01 , 0 . 02 , 0 . 03 , 0 . 04 , 0 . 05 , 0 . 06 , 0 . 07 , 0 . 08 , 0 . 09 . 0 . 10 , 0 . 11 } × { 50 , 60 , 70 , 80 , 90 , 100 , 110 , 120 , 130 , 140 , 150 } . We show our best results in bold. That is, our 'optimal' parameter values (see the sixth row) are τ = 0 . 04 and C ∗ = 60.

| τ      |   C ∗ | Error   | Error   | Error   |
|--------|-------|---------|---------|---------|
| 0 . 02 |    50 | 2 . 31  | ×       | 10 4    |
| 0 . 03 |    50 | 1 . 82  | ×       | 10 4    |
| 0 . 03 |    60 | 2 . 38  | ×       | 10 4    |
| 0 . 04 |    50 | 2 . 13  | ×       | 10 4    |
| 0 . 04 |    60 | 1 . 74  | ×       | 10 4    |
| 0 . 04 |    70 | 3 . 05  | ×       | 10 4    |
| 0 . 05 |    50 | 2 . 48  | ×       | 10 4    |
| 0 . 05 |    60 | 1 . 83  | ×       | 10 4    |
| 0 . 05 |    70 | 2 . 26  | ×       | 10 4    |
| 0 . 06 |    50 | 2 . 80  | ×       | 10 4    |
| 0 . 06 |    60 | 2 . 01  | ×       | 10 4    |
| 0 . 06 |    70 | 1 . 76  | ×       | 10 4    |
| 0 . 06 |    80 | 3 . 48  | ×       | 10 4    |
| 0 . 07 |    50 | 3 . 16  | ×       | 10 4    |
| 0 . 07 |    60 | 2 . 26  | ×       | 10 4    |
| 0 . 07 |    70 | 1 . 80  | ×       | 10 4    |
| 0 . 07 |    80 | 2 . 78  | ×       | 10 4    |
| 0 . 08 |    50 | 3 . 32  | ×       | 10 4    |
| 0 . 08 |    60 | 2 . 38  | ×       | 10 4    |
| 0 . 08 |    70 | 1 . 86  | ×       | 10 4    |
| 0 . 08 |    80 | 2 . 51  | ×       | 10 4    |
| 0 . 09 |    50 | 3 . 55  | ×       | 10 4    |
| 0 . 09 |    60 | 2 . 69  | ×       | 10 4    |
| 0 . 09 |    70 | 1 . 93  | ×       | 10 4    |
| 0 . 09 |    80 | 2 . 25  | ×       | 10 4    |
| 0 . 10 |    50 | 3 . 61  | ×       | 10 4    |
| 0 . 10 |    60 | 2 . 82  | ×       | 10 4    |
| 0 . 10 |    70 | 2 . 07  | ×       | 10 4    |
| 0 . 10 |    80 | 1 . 96  | ×       | 10 4    |
| 0 . 10 |    90 | 4 . 40  | ×       | 10 4    |
| 0 . 11 |    50 | 3 . 80  | ×       | 10 4    |
| 0 . 11 |    60 | 2 . 88  | ×       | 10 4    |
| 0 . 11 |    70 | 2 . 16  | ×       | 10 4    |
| 0 . 11 |    80 | 1 . 99  | ×       | 10 4    |
| 0 . 11 |    90 | 3 . 54  | ×       | 10 4    |

the individual stays in their current compartment. For compartments A and I, from which an individual can move to one of two possible new compartments, we draw transition times from Exp( χ 1 ) and Exp( χ 2 ), where χ 1 and χ 2 are the associated rate constants. If both times are less than 1 day, the individual moves to the compartment that has the smaller time. If only one of the times is less than 1 day, the individual moves to that compartment. If neither time is less than 1 day, the individual remains in their current compartment. When an individual moves to the I compartment, they may break their weak contacts. With probability b , they break all of their weak contacts; otherwise, they keep all of their weak contacts. Individuals in the I compartment become documented cases with probability τ . In our pseudocode, we refer to the breaking of contacts as 'deactivating' edges and refer to the re-establishment of contacts as 'reactivating' edges. If an individual moves to the H compartment, we deactivate all of their edges with weak and strong contacts. If an individual moves to the R compartment, we reactivate any edges that may have been deactivated because of their movement through the I and H compartments (except those that may not be active because (1) the other individual in the interaction is in the I compartment and did not break their weak connection or (2) the other individual is in the H compartment).

## B.2 Implementation of Approximate Truncated Power-Law Distributions

## B.2.1 Sampling from the Distribution

Given a lower bound a -, an upper bound a + , and an exponent p , we wish to approximate a power-law distribution for a discrete random variable N over [ a -, a + ], where Pr( N = n ) = O ( n -p ) as a + , n → ∞ . Our procedure amounts to (1) shifting the range to avoid the case a -= 0, (2) sampling from a continuous power-law probability density, (3) truncating the result to an integer, and (4) shifting the range back if we shifted the original range away from a -= 0. In our model, we use a -= 0 and a + = C ∗ , but we present the approach for a general finite sequence of nonnegative integers.

If a -= 0, we first shift to a distribution on [ A,B ], where A = max { a -, 1 } and B = a + +( A -a -). We define the normalization constant glyph[negationslash]

<!-- formula-not-decoded -->

To choose N , we select u ∈ [0 , 1) from a uniform distribution and select x ∗ such that

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

where glyph[floorleft] z glyph[floorright] is the floor of z (i.e., the largest integer that is less than or equal to z ). That is, glyph[negationslash]

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

glyph[negationslash]

We then calculate

Finally, we shift back to set

.

Note that

<!-- formula-not-decoded -->

glyph[negationslash]

## Algorithm 1 A Simulation of the Spread of COVID-19 on a Contact Network

Input:

```
A set of values for each parameter that we list in Table 1
```

Output: Daily counts of people in each compartment; documented cases

- 1: Initialize Population of size P Ottawa with fractions f dis who are disabled, f care who are caregivers, f ess who are essential workers, and f gen who are members of the general population. At initialization, we determine whether or not each individual will break all of their weak contacts if they become ill (they break weak contacts with probability b ) and determine whether or not they will have a positive test result if they become ill (a positive test occurs with probability τ ).
- 2: Assign a unique integer ID to each individual in Population .
- 3: Obtain WeakStubs from Algorithm 2 with input Population .
- 4: Obtain PossibleHouseholdUnits from Algorithm 3 with input Population .
- 5: Assign weak contacts using Algorithm 4 with inputs Population, WeakStubs .
- 6: Assign strong contacts using Algorithm 5 with inputs Population, PossibleHouseholdUnits .
- 7: Match disabled people and caregivers using Algorithm 6 with input DisabledPopulation , where DisabledPopulation refers to all individuals in Population who are in the disabled subpopulation.
- 8: Initialize some number of people to be asymptomatic or ill based on program inputs. (In all of our simulations in the present paper, we initialize these individuals to be asymptomatic, but one can instead use our code to initialize individuals as ill; one can also initialize some individuals to be asymptomatic and some individuals to be ill.)
- 9: day = 0, has opened = false , has closed = false
- 11: Compute the number of individuals from each subpopulation in each compartment, as well as the number of documented cases from each subpopulation.

```
10: while day < end day do 12: for each disabled individual in DisabledPopulation do 13: Select a weak caregiver uniformly at random from their set of weak caregivers. 14: end for 15: for each individual in Population do 16: Calculate the infection probability using Algorithm 7 with input individual . 17: end for 18: for each individual in Population do 19: Advance state by 1 day using Algorithm 8 with input individual . 20: end for 21: day ← day +1 22: if time < close time then 23: Do nothing. 24: else if time < open time then 25: if not has closed then 26: Close down (i.e., start a lockdown) using Algorithm 9. 27: has closed ← true 28: end if 29: else 30: if not has opened then 31: Reopen (i.e., end a lockdown) using Algorithm 10. 32: has opened ← true 33: end if 34: end if 35: end while
```

## Algorithm 2 Weak Stubs

Input: A container of nodes (which we denote by Population )

Output: A container of IDs (which we denote by WeakStubs ) in which the ID of each node in Population occurs with a multiplicity that is equal to the number of stubs of that node.

- 1: for each individual in Population do
- 2: Let target equal the number of weak stubs that individual can potentially have; we draw this number from D group,period , where 'group' is their subpopulation and 'period' is the current state of the pandemic (prelockdown, lockdown, or post-lockdown).
- 3: Let current equal the number of current weak stubs of individual .
- 4: if current &lt; target then
- 5: needed = current -target
- 6: else 7: needed = 0 8: end if 9: For needed number of times, append the ID of individual to a container WeakStubs . 10: return WeakStubs
- 11: end for

## Algorithm 3 Household Units

Input: A container of nodes (which we denote by Population )

Output: Acontainer of containers of IDs (which we denote by PossibleHouseholdUnits )

- 1: Let AllIDs be a container that stores the unique ID for each individual in Population
- 2: while AllIDs not empty do
- 3: Choose an ID, which we denote by ID 1 , uniformly at random from AllIDs and determine the number of household contacts ( house ) of the individual with that ID by sampling from D strong .
- 4: Select house number of IDs uniformly at random from AllIDs .
- 5: Append ID 1 and the above IDs to a container unit.
- 6: Remove all of the IDs in unit from AllIDs .
- 7: end while
- 8: return PossibleHouseholdUnits (which is a container that holds each unit )

## Algorithm 4 Assigning Weak Contacts

Input: A container of nodes (which we denote by Population ) and a container of IDs (which we denote by WeakStubs )

Result: All nodes in Population are assigned weak contacts

- 1: while | WeakStubs | ≥ 2 do
- 2: Choose IDs ID 1 and ID 2 uniformly at random from WeakStubs .

glyph[negationslash]

- 3: if ID 1 = ID 2 and the individuals with IDs ID 1 and ID 2 are not already contacts (weak, strong, or caregiving) then
- 4: Make the individuals with IDs ID 1 and ID 2 into weak contacts.
- 5: end if
- 6: Remove ID 1 and ID 2 from WeakStubs .
- 7: end while

## Algorithm 5 Assigning Strong Contacts

Input: A container of nodes (which we denote by Population ) and a container of containers of IDs (which we denote by PossibleHouseholdUnits )

Result: All nodes in Population are assigned strong contacts

- 1: for each unit in PossibleHouseholdUnits do
- 2: for each ID in unit do
- 3: Make the individual with ID a strong contact of each other member of the unit, unless the individuals are already contacts (weak, strong, or caregiving).
- 4: end for
- 5: end for

## Algorithm 6 Matching Disabled People and Caregivers

Input: A container of nodes (which we denote by DisabledPopultion ) that all belong to the same subpopulation

Result: All nodes in DisabledPopulation are assigned a pool of weak caregivers and one strong caregiver

- 1: for each disabled individual in DisabledPopulation do
- 2: Determine care weak num from D pool , which is the number of weak caregivers in their pool.
- 3: Select care weak num number of caregivers uniformly at random from the set of caregivers and store them in CaregiversChosen .
- 4: for each caregiver in CaregiversChosen do
- 5: if disabled individual and caregiver are not already contacts (weak, strong, or caregiving) then
- 6: Make their relationship a weak caregiver-disabled relationship.
- 7: end if
- 8: end for
- 9: end for
- 10: for each disabled individual in DisabledPopulation do
- 11: Choose 1 caregiver uniformly at random from the set of caregivers.
- 12: if disabled individual and the caregiver are not already contacts (weak, strong, or caregiving) then
- 13: Make their relationship a strong caregiver-disabled relationship.
- 14: end if
- 15: end for

## Algorithm 7 Infection Probability

```
Input: A node (which we denote by individual ) Output: An infection probability (which we denote by infect prob ) 1: not get = 1 2: if individual is Susceptible then 3: for each weak contact in individual 's weak contacts do 4: if edge to weak contact is active and weak contact is contagious then 5: if both wear a mask then 6: not get ← not get × (1 -βmw w ) 7: else 8: not get ← not get × (1 -βw w ) 9: end if 10: end if 11: end for 12: for each strong contact in individual 's strong contacts do 13: if edge to strong contact is active and strong contact is contagious then 14: not get ← not get × (1 -βw s ) 15: end if 16: end for 17: if individual is disabled then 18: for each caregiver in their set of weak caregivers for the day do 19: if edge to caregiver is active and caregiver is contagious then 20: if both wear a mask then 21: not get ← not get × (1 -βmw c ) 22: else if one wears a mask then 23: not get ← not get × (1 -β √ mw c ) 24: else 25: not get ← not get × (1 -βw c ) 26: end if 27: end if 28: end for 29: if edge to individual 's strong caregiver is active and the strong caregiver is contagious then 30: if both wear a mask then 31: not get ← not get × (1 -βmw c ) 32: else if one wears mask then 33: not get ← not get × (1 -β √ mw c ) 34: else 35: not get ← not get × (1 -βw c ) 36: end if 37: end if 38: else if individual is a caregiver then 39: for each disabled individual in their set of disabled contacts for the day do 40: if edge to disabled individual is active and disabled individual is contagious then 41: if both wear a mask then 42: not get ← not get × (1 -βmw c ) 43: else if one wears mask then 44: not get ← not get × (1 -β √ mw c ) 45: else 46: not get ← not get × (1 -βw c ) 47: end if 48: end if 49: end for 50: end if 51: end if 52: infect prob = 1 -not get 53: return infect prob
```

## Algorithm 8 Advancing One Day

```
Input: A node (which we denote by individual ) Result: individual remains in their current compartment or moves to a new one 1: if individual is in the S compartment then 2: In the time interval ∆ T = 1 day, move individual into the E compartment with probability infect prob. 3: else if individual is in the E compartment then 4: Sample T asymptomatic from the distribution Exp( ν ). 5: if T asymptomatic < 1 day then 6: Move individual to the A compartment. 7: end if 8: else if individual is in the A compartment then 9: Sample T ill from Exp( α ). 10: Sample T removed from Exp( η ). 11: if T ill < T removed then 12: if T ill < 1 day then 13: Move individual to the I compartment. 14: Deactivate all of their edges to weak contacts if that ill individual is one who breaks their weak contacts. 15: end if 16: else 17: if T removed < 1 day then 18: Move individual to the R compartment. 19: Reactivate all of their edges to weak contacts (provided either that the weak contact has no symptoms or that the weak contact is in the I compartment but does not break weak contacts when ill). 20: end if 21: end if 22: else if individual is in the I compartment then 23: Sample T hospital from Exp( µ ). 24: Sample T removed from Exp( ρ ). 25: if T hospital < T removed then 26: if T hospital < 1 day then 27: Move individual to the H compartment. 28: Deactivate all of their edges to weak and strong contacts. 29: end if 30: else 31: if T removed < 1 day then 32: Move individual to the R compartment. 33: Reactivate all of their edges to weak contacts (provided either that the weak contact has no symptoms or that the weak contact is in the I compartment but does not break weak contacts when ill). 34: end if 35: end if 36: else if individual is in the H compartment then 37: Sample T removed from Exp( ζ ). 38: if T removed < 1 day then 39: Move individual to to the R compartment. 40: Reactivate all of their edges to weak contacts (provided either that the weak contact has no symptoms or that the weak contact is in the I compartment but does not break weak contacts when ill). 41: end if 42: end if
```

## Algorithm 9 Closing Down (i.e., starting a lockdown)

Result: Lockdown mask-wearing strategies and contact-limiting strategies are implemented for each node in Popu-

- 3: Determine their new number of weak contacts by sampling new target value from D group,post , where group is the subpopulation of the individual.
- Input: A container of nodes (which we denote by Population ) lation 1: Update mask-wearing statuses. 2: for each individual in Population do 4: end for 5: for each individual in Population do 6: clear = max { 0 , current weak contacts -new target value } 7: i = 0 8: while i &lt; clear do 9: Select a weak contact glyph[pi1] uniformly at random. 10: if neither glyph[pi1] nor individual is an essential worker then 11: Remove the edge between the nodes. 12: end if 13: i ← i +1 14: end while 15: end for

## Algorithm 10 Reopening (i.e., ending a lockdown)

Input: A container of nodes (which we denote by Population )

Output: Reopening mask-wearing strategies and contact-limiting strategies are implemented for each node in Population

- 1: Update mask-wearing statuses.
- 2: Obtain a container new weak stubs by applying Algorithm 2 with input Population
- 3: Apply Algorithm 4 with inputs Population and new weak stubs .

thereby ensuring that asymptotically we have a power law as n →∞ .

## B.2.2 Estimating the Mean

When B -A is large, it can be computationally expensive to compute the precise mean of the random variable N = n ∗ -( A -a -) that we obtain from Eqs. (4)-(7). When B -A is large, it is also the case that rounding errors and overflow errors can cause an estimation of the true mean to be inaccurate. Therefore, we estimate the mean analytically. Given a -, a + , and p , we seek to estimate the mean E p := E ( N ) over the interval [ a -, a + ].

We have that glyph[negationslash]

glyph[negationslash]

<!-- formula-not-decoded -->

To obtain the third equality, we rewrote ∑ B n = A n ( n + 1) 1 -p as ∑ B +1 n = A +1 ( n -1) n 1 -p , whose n 2 -p terms cancel with ∑ B n = A n 2 -p except at n = A and n = B +1.

For our approximation, we consider multiple cases.

p = 1 : Note that { n log( n +1 n ) } B n = A is an increasing sequence of terms. Therefore,

<!-- formula-not-decoded -->

Because ∫ x log( x +1 x )d x = 1 2 ( x 2 log(( x +1) /x ) + x -log( x +1)) + const, we compute the integrals exactly and obtain the estimate E 1 = C -1 ( S 1 + S 1 ) / 2.

p = 2 : We need to estimate ∑ B +1 n = A +1 n -1 . Because the sequence 1 /n is decreasing,

<!-- formula-not-decoded -->

We then estimate E 2 = C -1 ( 1 2 ( S 2 + S 2 ) ) , where A 2 -p = ( B +1) 2 -p = 1 allows us to cancel terms.

p / ∈ { 1 , 2 } , p &gt; 1 : We need to estimate ∑ B +1 n = A +1 n 1 -p , where the terms are decreasing. Therefore,

<!-- formula-not-decoded -->

We then estimate E p&gt; = ((1 -p ) C ) -1 ( ( B +1) 2 -p -A 2 -p -1 2 ( S p&gt; + S p&gt; ) ) .

A

Law

Power

20

le estimated and exact mean values of the annroximate truncated nower-law distributic

---==

Exact

0.00

Fig 12. (A) The estimated and exact mean values of the approximate truncated power-law distribution for various values of p . The curves are indistinguishable. (B) The error in computing the mean for our approximations. In this figure (both panel A and panel B), we use a -= 0 and a + = 100.

<!-- image -->

p / ∈ { 1 , 2 } , p &lt; 1 : We need to estimate ∑ B +1 n = A +1 n 1 -p , where the terms are increasing. Therefore,

<!-- formula-not-decoded -->

We then estimate E p&lt; = ((1 -p ) C ) -1 ( ( B +1) 2 -p -A 2 -p -1 2 ( S p&lt; + S p&lt; ) ) .

This approximation is very accurate. When a -= 0 and a + = 100, we plot the approximations and the numerically exact values in Fig. 12.

## C Additional Computational Experiments

## C.1 Examining a Distribution with a Deterministic Number of Weak Contacts

The confidence window for the cumulative documented case counts is large. To determine the cause of this large variance, we run trials (see Fig. 13) in which each subpopulation has a deterministic number of weak contacts that is equal to the mean values in Section 2.2. When the weak-contact distribution is deterministic, we find that the variance in documented cases is much smaller than when weak contacts are distributed according to an approximate truncated power-law. Additionally, the using the deterministic distribution results in many fewer cases of the disease, which hardly spreads.

## C.2 Different Values of the Caregiver-Disabled Edge Weight w c

The risk of COVID-19 infections in a caregiver-disabled interaction is larger than in an ordinary household interaction. In Fig. 14, we compare our results for two different values of the caregiver-disabled edge weight w c . The choice w c = 1 results in essential workers, who have many weak contacts, being the most potent disease spreaders among all subpopulations (except for spreading from caregivers to other caregivers). However, even the choice w c = 1 . 5 (which is smaller than the value w c = 2 . 27 that we used in most of our computations) results in caregivers being the most potent spreaders of the disease to the disabled subpopulation.

Estimated

B

Subpopulation Infected Through Day 148

3000

Caregiver

8 2500 -

Approximate Truncated Power-Law Distribution

Fractions Infected Through Day 148

W. = 1.0

95% of Simulations

0.01267

Stochastic-Model Mean

- - - Lockdown Strategies Enacted nte

2000 -

Disabled

1500 -

Docume

Essential Worker

0.02245

0.02048

0.0112

0.01798

500-

General

0.0102

10 Feb. 2020

24 Mar. 2020

General

• 13. Comnarison of a. mean of 100 simulations when weak contacts are distributed accordine to an anoroximat.

2500 -

2000-

0.01969

0.0186

Deterministic Distribution w. = 1.5

3000 -

0.03214

0.016

- 0.045

- 0.04

- 0.035

- 0.03

Caregiver

Disabled

0.01598

0.01234

0.02881

0.03085

0.02224

0.02381

0.04523

0.02461

0.045

0.04

0.035

0.03

<!-- image -->

Essential Worker

Fig 13. Comparison of a mean of 100 simulations when weak contacts are distributed according to an approximate truncated power-law distribution and a deterministic distribution. In both plots, the mean is depicted in blue and the gray window indicates the middle 95% of these 100 simulations. On day 44 (24 March, 2020), all groups limit contacts and all individuals in caregiver-disabled interactions and interactions between essential workers and their weak contacts wear masks.

Fig 14. Fraction of each subpopulation that is infected through day 148 when all of the initially infected individuals are in a single subpopulation for (left) w c = 1 and (right) w c = 1 . 5. On day 44, all groups limit contacts and disabled people, caregivers, and essential workers wear masks.

<!-- image -->

Cumulative Documented Cases

Subpopulation Infected Through Day 148