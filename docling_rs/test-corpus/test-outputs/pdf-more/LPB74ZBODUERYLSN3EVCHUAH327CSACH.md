<!-- image -->

Forest Service

Rocky Mountain Research Station

Fort Collins, Colorado 80526

Proceedings RMRS-P-13

March 2000

<!-- image -->

<!-- image -->

## Land Stewardship in the 21st Century: The Contributions of Watershed Management

## Simulating Soil Moisture Change in a Semiarid Rangeland Watershed with a Process-Based Water-Balance Model

## Howard Evan Canfield1 and Vicente L. Lopes2

Abstract.-A process-based, simulation model for evaporation, soil water and stream/low (BROOK905) was used to estimate soil moisture change on a semiarid rangeland watershed in southeastern'Arizona. A sensitivity analysis was performed to select parameters affecting ET and soil moisture for calibration. Automatic parameter calibration was performed using a procedure based on a GLYPH(cmap:df00)auss-type downhill search algorithm and a least squares objective function. Results indicated that BROOK90 can be used to simulate changes in soil moisture content in the upper 15 cm on semiarid rangeland environments, an important realization for watershed management in the southwest.

## Introduction

Annual rainfall variability tends to increase with in creasing aridity so that the coefficient of variation for annual rainfall tends to be higher in semiarid environ ments. In southeastern Arizona, rates of evapotranspiration (ET) are high, and soils tend to be dry. Winter rainfall tends to be less intense than the summer monsoons. Therefore, vegetation is under greater stress during the summer when rates of potential evapotranspiration are much higher than the actual transpiration.

The vegetation production and subsequent capacity of the land to support grazing therefore depends on rainfall that may vary significantly from year to year. By better understanding changes in soil moisture, it may be pos sible to improve the management of rangelands by reduc ing the number of grazing animals when soil moisture is low and plant stress is high.

In this study, soil moisture change was monitored across two very different years; one in which annual rainfall was high, and a second in which annual rainfall was low. These data were collected at different depths beneath vegetation and under bare soil conditions to improve understanding of the effect of vegetation on soil moisture.

' Department of Agricultural and Biosystems Engineering, University ofArizona, Tucson, AZ

7SchoolofRenewableNaturalResources, UniversityofArizona, Tucson, AZ

3 Trade names are used for the information and convenience of the reader and do not imply endorsement or preferential treatment by the sponsoring organizations or the USDA Forest Service.

## Objectives

The objectives of this study were to: 1) compare soil moisture variability temporally and with soil depth under bare and vegetated conditions across a dry and a wet year, and 2) determine if soil moisture can be modeled using a soil moisture accounting model, which might then be used as a management tool for estimating soil moisture and subsequent plant stress.

## Methods

## Soil Moisture Measurements

Volumetric soil moisture content was measured using time domain reflectometry (TDR) probes installed hori zontally in pits located on the northern edge of the Lucky Hills 104 watershed. This watershed is operated by the USDA Agricultural Research Service, Walnut GLYPH(cmap:df00)ulch Ex perimental Watershed near Tombstone, AZ. Six pits were dug, three under desert shrub (shrub), and three in un shaded locations (bare). Probes were installed at depths of 5cm, 10cm, 15cm, 20 cm, 30 cm and 50 cm. The probes measured the volumetric moisture content averaging the 2.5 cm above and below the actual measurement point. Soil moisture data collected on the watershed in 1990 and 1991 were studied. For much of the monsoon in 1990 and 1991, soil moisture was sampled daily, decreasing toevery 3 to 7 days by the end of the monsoon.

## BROOK90 Model

The BROOK90model(Federer, 1995) was used to model soil moisture. The model has a strong physically-based description of ET for sparse canopies (Shuttleworth and Wallace, 1985) and redistribution of infiltrated water (Clapp and Hornberger, 1978).

Initial parameter values for the Shuttleworth-Wallace (1985) relationship were estimated using values compiled by Federer et. al. (1996) for xeric shrub. Some minor

'■'A'f's

'

§

-

■

j

modifications were made to reflect field observations. Clappand Hornberger(1978)describe soil moisture move ment as a non-linear function of soil wetness. Federer (1995) provides estimates for soil parameters at field ca pacity for the USDA soil textural classes.

In this study, net precipitation (i.e. measured precipita tion - measured runoff), rather than total precipitation was used, so that BROOK90 operated only as a soilmoisture accounting model, rather than a rainfall-runoff model. Daily temperature data for Tombstone were used. Daily total horizontal solar radiation measured at Fort Huachuca wasused. Vapor pressure was calculated using average daily dew point temperature recorded at Tucson, and daily wind speed wasapproximated using the monthly averages recorded at Tucson.

## Sensitivity Analysis

A sensitivity analysis was performed to select param eters affecting ET and soil moisture. An initial set of ET parameters based on default values included in BROOK90 (Federer, 1995; Federer et al. 1996) were used. Canopy parameters were estimated based on initial default val ues. The upper and lower bounds for Clapp and Hornberger (1978) soil parameters were set at one stan dard deviation above or below the mean value for sandy loam or loamy sand based on the work of Li et. al (1976). Values of porosity were allowed to vary over the range of values determined by Whitaker (1993). The parameter values included the upper and lower value from the literature for any canopy type.

## Parameter Estimation

Since data exists for six layers and ten parameters were modified, numerous possibilities of different parameter combination are possible. For this reason, a parameter estimation program called PEST (Watermark Software) was used to estimate optimal parameter values. This program uses a GLYPH(cmap:df00)auss-type downhill search routine (Marquardt 1963). The objective function is based on a least-squares criterion and the convergence criterion is based largely on user choices.

## Results and Discussion

## Soil Moisture Measurements

The active depth of infiltration was estimated based on the observed volumetric moisture data collected in 1990

and 1991 for the shrub and bare conditions. Figure la shows a plot of volumetric soil moisture vs. day of the year for days 200 to 230 of 1990 for bare soil condition based on average values for the three sample pits. Figure lb shows the shows soil moisture for the same period from the three pits under shrubs. For shrub condition, soil moisture seems to influence the upper 15 cm. In contrast, soil moisture changes in the 20cm, 30cm, and 50cm'depths under shrubs are more gradual and changed very little on a rainfall day. Based on this observation, the top 15 cm were assumed to be the zone of active infiltration on a rainfall day. Under "bare" conditions, the infiltrated depth could be as deep as 20 cm. In 1991, soil moisture changes were similar to 1990, but in the deeper profile there was very little change throughout the summer.

In fact, soil moisture in these two years was found to be very different, especially deeper in the profile. The soil moisture observations for 1990 and 1991 show that the soil is more moist in 1990. Furthermore, average volumetric soil moisture is significantly higher in 1990 for 30cm + 50 cm measurements (16% vs. 9.1% for 1991 at the 0.025 level of significance).

Figure 1a. Observed volumetric soil moisture under bare cover.

<!-- image -->

Figure 1b. Observed volumetric soil moisture under shrub.

<!-- image -->

## Sensitivity Analysis

Results ofthe sensitivity analysis summarized in table 1 show that BROOK90 is sensitive to both soil and ET parameters (shaded rows indicate canopy parameters). The model is most sensitive to canopy density, volumetric moisture content at field capacity, maximum plant con ductivity, maximum leaf area index, exponent on soil ten sion, soil evaporation resistance at field capacity, exponentof soil evaporation to water potential, matric potential at field capacity maximum leaf conductance, and hydraulic conductivity at field capacity. It is relatively insensitive to albedo, relative distribution of rainfall in the top three layers, allowing or disallowing deep drainage, and porosity.

## Parameter Estimation

Among the difficulties encountered during parameter estimation were an inability to find the same set of param eter values, unrealistic parameter combinations, large errors in simulated vs. measured soil moisture for some layers, large errors toward the end of the simulation period where measurement were less frequent, and unrealistic changes in parameters from gauged to ungaged soil layers.

While a unique set of parameter values could not be obtained, measures of model efficiency indicated that the simulations were good with little bias. Model efficiencies exceeded 0.75 for both years, and there appears to be no systematic bias in the estimate of soil moisture in the upper 15 cm. Figure 2a shows a plot of the simulation and observed values for the top 15 cm of the profile (layers 13 of the simulation) for days 200 to 300 of 1990. Figure 2b

Figure 2a. Simulated and observed volumetric moisture content 1990 (0-15cm).

<!-- image -->

Figure 2b. Simulated and observed volumetric soil moisture 1991 (0-15cm).

<!-- image -->

Table 1. Results of Sensitivity Analysis. Shaded lines are for soil parameters. ; are for vegetation parameters affecting evapotranspiration . Unshaded lines

|                                                           | UNI 11> initial Kenuroed Mean Maximum h'enurDea lOTean Maximum   | UNI 11> initial Kenuroed Mean Maximum h'enurDea lOTean Maximum   | UNI 11> initial Kenuroed Mean Maximum h'enurDea lOTean Maximum   | UNI 11> initial Kenuroed Mean Maximum h'enurDea lOTean Maximum   | UNI 11> initial Kenuroed Mean Maximum h'enurDea lOTean Maximum   | UNI 11> initial Kenuroed Mean Maximum h'enurDea lOTean Maximum   | UNI 11> initial Kenuroed Mean Maximum h'enurDea lOTean Maximum   | UNI 11> initial Kenuroed Mean Maximum h'enurDea lOTean Maximum   |
|-----------------------------------------------------------|------------------------------------------------------------------|------------------------------------------------------------------|------------------------------------------------------------------|------------------------------------------------------------------|------------------------------------------------------------------|------------------------------------------------------------------|------------------------------------------------------------------|------------------------------------------------------------------|
| (^nqpyjUens.ity« f ^ ^ '"■"" '"""'v:.^                    |                                                                  |                                                                  |                                                                  |                                                                  |                                                                  |                                                                  |                                                                  | [7T03&~::;i                                                      |
| I heta at Meld Capacity                                   |                                                                  | 0.197a                                                           | 0.175                                                            | -11%                                                             | -25%                                                             | 0.230                                                            | 17%                                                              | 40%                                                              |
| Maximjiriiwm^ge&mexim) 7 : ;^ bxponent on Soil lension(b) | Imm/gj                                                           | ' -B; j ['»■ !                                                   | - SIS':. in ■ faiT                                               |                                                                  | L;. ^ ;>j                                                        |                                                                  |                                                                  |                                                                  |
|                                                           |                                                                  |                                                                  |                                                                  |                                                                  |                                                                  | 6.70                                                             |                                                                  |                                                                  |
| ^pjiejjiipipojig^pjEjratioiiii1"- .^^h/-;?                |                                                                  | 5.72a !:■■'. ir i                                                | 3.20 I'-'■■&&■!                                                  | -15% fitfMi1,'                                                   | -49%                                                             |                                                                  | 5%                                                               | 14%                                                              |
| Matric Potential at Field Capacity                        | (kPa)                                                            | -9.17a                                                           | -4.70                                                            | " 0%                                                             | -21%                                                             | -24.00                                                           | 6%                                                               | 17%                                                              |
| Maximum%eatufe*pn^uQtariGLYPH(cmap:df00)§ ■ : ■           | ■;{&#£)■                                                         | I 0.53                                                           | 0:80                                                             | -6%                                                              | -18% '                                                           | 0i5b'-                                                           | t 1%                                                             | 5% "■'.                                                          |
| Hydraulic Conductivity at hield Capacity                  | (mm/d)                                                           | 2                                                                | 4                                                                | 0%                                                               | 11%                                                              | 1                                                                | 3%                                                               | 11%                                                              |
|                                                           |                                                                  | 0;1617                                                           | i0i1400                                                          | 0%                                                               | -2%                                                              | 0.2600                                                           | 2%-                                                              | - 7%                                                             |
| Distribution or infiltration in 1 op ^Layers              |                                                                  |                                                                  | i 0.5                                                            |                                                                  |                                                                  | ,                                                                | 1%                                                               | :                                                                |
| Deep Drainage                                             |                                                                  | 0.72                                                             |                                                                  | -1%                                                              | -2%                                                              | 1                                                                | 0%                                                               | 3%                                                               |
|                                                           |                                                                  | 0                                                                | 1                                                                | -1%                                                              | -5%                                                              | 0.2                                                              |                                                                  | -1%                                                              |
| Porosity                                                  |                                                                  | 0.414a                                                           | 0.350                                                            | 0%                                                               | -1%                                                              | 0.450                                                            | 0%                                                               | 1%                                                               |

shows those same plots for the simulation averaging the values for 1991. For comparison purposes, the simulated values are also plotted against the observed values, and the Nash and Suttcliffe (1970) model efficiency is used to describe the goodness of fit.

The model did not perform as well in estimating soil moisture in the lower portion of the soil profile as mea sured by the 30 cm + 50 cm volumetric soil moisture. The simulation for the wetter year (1990) was reasonably good as indicated by a model efficiency statistic of 0.53. How ever, the simulation was poor at the 30cm + 50cm depth as indicated by a -5.68 model efficiency in the drier year (1991) in part because the observed values of soil moisture did not change markedly at those depths in 1991.

## Conclusions

The observed soil moisture in two subsequent years varied considerably in a semiarid rangeland watershed in southeastern Arizona. While soil moisture in the upper 15 cm showed no statistical difference in the two years, the soil moisture in the 30cm to 50cm depths varied consider ably. This indicates that great soil moisture variability is expected to occur deeper in the profile from a wetter to a drier year. This suggests that the occurrence of summer rainfall may have a stronger effect on shallow-rooted vegetation and less effect on deep-rooted vegetation sys tems.

The significant overall variability of soil moisture be tween the two years presented a modeling difficulty. Calibration and simulation results showed that BROOK90 can be used to estimate soil moisture in the first 15 cm, but performed poorly insimulatingsoil water at deeper layers in the soil profile. Little systematic error was noted and fitted parameter values were within what can be consid ered reasonable for a sandy loam soil, which suggests that BROOK90 can be used to simulate changes in soil moisture content in the upper 15 cm on semiarid rangeland watersheds. Results from this study, therefore, sug gest that the model mightbe used to simulate soil moisture in the upper portion of the soil profiles, an important realization for watershed management in the southwest.

## Acknowledgments

The Authors would like to thank Mr. Tim Keefer of the USDA Agricultural Research Service, Southwest Water shed Research Center inTucson, AZ, for providing the soil moisture data used in this study. Dr. Donald Slack and Dr. William Rasmussen of the Department of Agricultural and Biosystems Engineering at the University of Arizona reviewed the paper.

## Literature Cited

- Clapp, R.B. and Hornberger, GLYPH(cmap:df00).M. 1978. Emperical equa tions for some soil hydraulic properties, Water Re sources Research, Vol. 14., No. 4., 601-604
- Federer, C.A. 1995. BROOK90. A Simulation Model for Evaporation, Soil Water, and Streamflow. Version 3.1 Computer Freeware and Documentation. USDA Forest Service, P.O. Box 640, Durham, N.H. 03824
- Federer, C.A., Vorosmarty, C. and Fekete, B. 1996. Intercomparison of methods for calculating portential evaporation in regional and global water balances, Water Resources Research, Vol. 32., No. 7., 2315-2321
- Li, E.A., Schanholz, V.O. and Carson, E.W. 1976. Estimat ing Saturated Hydraulic Conductivity and Capillary Potential at the Wetting Front. Dept of Agr. Eng. Virginia Polytech Inst. and State University, Blacksburg, Va.
- Marquartdt, D.W. 1963. An algorithm for least-squares estimation of nonliear parametersjournal of the Soci ety of Industrial and Applied Mathematics, Vol. 11, No. 2,431-441.
- Nash, J.E. and Sutcliffe, J.V. 1970. River flow forcasting through conceptual models, I.A discussion of prin ciples. Journal of Hydrology, Vol. 10,282-290.
- Shuttleworth, W.J. and Wallace, J.S. 1985. Evaporation fromsparsecrops-anenergy combination theory, Quar terly Journal of the Royal Meteorological Society, Vol. Ill, 839-855.
- Whitaker, M.P.L., 1993. Small-scale spatial variability of soil moisture and hydraulic conductivity in a semi-arid rangeland soil in Arizona. Unpublished Master of Sci ence Thesis. University of Arizona.