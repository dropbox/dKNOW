## Highlights

## Nowcasting Stock Implied Volatility with Twitter

Thomas Dierckx , Jesse Davis , Wim Schoutens

- GLYPH&lt;136&gt; Next-day movements in stock implied volatility can be predicted using random forests.
- GLYPH&lt;136&gt; Attention and sentiment features extracted from Twitter improve predictive performance.
- GLYPH&lt;136&gt; Predictive performance varies significantly across the 11 traditional stock market sectors.
- GLYPH&lt;136&gt; Implied volatility regimes identified by hidden Markov models provide actionable insights on when the proposed approach works best per stock market sector.

## Abstract

In this study, we predict next-day movements of stock end-of-day implied volatility using random forests. Through an ablation study, we examine the usefulness of di GLYPH&lt;11&gt; erent sources of predictors and expose the value of attention and sentiment features extracted from Twitter. We study the approach on a stock universe comprised of the 165 most liquid US stocks diversified across the 11 traditional market sectors using a sizeable out-of-sample period spanning over six years. In doing so, we uncover that stocks in certain sectors, such as Consumer Discretionary, Technology, Real Estate, and Utilities are easier to predict than others. Further analysis shows that possible reasons for these discrepancies might be caused by either excess social media attention or low option liquidity. Lastly, we explore how our proposed approach fares throughout time by identifying four underlying market regimes in implied volatility using hidden Markov models. We find that most added value is achieved in regimes associated with lower implied volatility, but optimal regimes vary per market sector.

## 1. Introduction

Today's age is characterized by an ever-increasing connected and opinionated world. The widespread adoption of social media has caused significant changes in the world across many domains, and more are probably to follow. In the case of financial markets, participants now have access to countless online platforms to share their thoughts and feelings on certain events. Proponents of the E GLYPH&lt;14&gt; cient Market Hypothesis (Fama, 1970) ought to be pleased. The advent of mass media facilitates rapid information di GLYPH&lt;11&gt; usion, possibly propelling markets into a higher tier of price e GLYPH&lt;14&gt; ciency. However, behavioral economists would argue that this type of media might very well influence investors and incite herd behavior which in turn induces ine GLYPH&lt;14&gt; ciency (e.g. Baker and Wurgler, 2006; Chiang and Zheng, 2010). Theory aside, this new wealth of information has not escaped the notice of the financial establishment. Indeed, data providers such as Bloomberg and Refinitiv now o GLYPH&lt;11&gt; er extensive social media indicators to help financial institutions navigate this new world. Although the competitive edge that resides in these alternative data sources remains veiled in secrecy, an abundance of academic studies have already tried

? Declarations of interest: none

GLYPH&lt;3&gt; Corresponding author

Email addresses: thdierckx@gmail.com (Thomas Dierckx ), jesse.davis@kuleuven.be (Jesse Davis ), wim.schoutens@kuleuven.be (Wim Schoutens )

## Nowcasting Stock Implied Volatility with Twitter

Thomas Dierckx a,b, GLYPH&lt;3&gt; , Jesse Davis b , Wim Schoutens a a Department of Statistics and Risk, KU Leuven, Celestijnenlaan 200B, Leuven, 3000, Belgium b Department of Computer Science, KU Leuven, Celestijnenlaan 200A, Leuven, 3000, Belgium

quantifying the interplay between social media and certain financial variables, providing insight into the predictive power of the masses.

Existing research has mainly focused on the Twitter platform and its influence on three prominent financial variables: stock price (e.g. Groß-Klußmann et al., 2019; Schnaubelt et al., 2020), realized volatility (e.g. Karagozoglu and Fabozzi, 2017), trading volume (e.g. Guijarro et al., 2019) or a combination thereof (e.g. Oliveira et al., 2017; Li et al., 2018). Remarkably, current literature completely overlooks the interaction between social media and the market implied volatility of stocks. Derived from option prices, this variable is deemed to be one of the more important parameters in the world of derivatives. In contrast to historical volatility, this is a forward-looking metric that indicates how much risk the market expects a certain asset to exhibit in the coming period. As this variable serves as a proxy for both market sentiment and option contract prices, the ability to predict its movements would be advantageous for the practice of asset management and market making alike.

Most prevailing studies in this domain have two important methodological shortcomings. First, analysis is typically performed on either a handful of arbitrarily chosen stocks or indices tracking the entire market (e.g. Groß-Klußmann et al., 2019), omitting sector idiosyncrasies in the process. Second, hypotheses are commonly tested on a relatively small time window ranging from a month (e.g. Bollen et al., 2011) to a few years (e.g. Schnaubelt et al., 2020). However, the ever-changing nature of financial markets warrants a closer look into how the interplay between social media patterns and financial variables evolves over longer periods of time. As patterns may emerge and dissipate over time, a crucial aspect of analysis is often left out.

The contribution of this study is threefold. First, to the best of our knowledge, we are the first to investigate to what extent a stock its one-day ahead movement in implied volatility can be predicted using machine learning on di GLYPH&lt;11&gt; erent combinations of feature sources, including Twitter. Second, instead of arbitrarily choosing a handful of stocks for our study, we diversified our stock universe across the 11 traditional US stock market sectors yielding 165 stocks in total. This allowed us to measure and explore the variability in predictive performance present among sectors. Lastly, we examined predictive performance on an out-of-sample period spanning January 1st, 2013 till March 1st, 2019. This period is significantly larger than many other studies and gave us the opportunity to not only be more robust, but also examine predictive performance throughout time. Instead of performing a year by year analysis of predictive performance, we used hidden Markov models to identify four regimes in the implied volatility of a stock and gauged whether performance varies across them. We argue that this alternative quantification of time yields more actionable insight as it allows practitioners to better anticipate future performance.

## 2. Preliminaries

This section presents background information on the key components used in this study. First, Section 2.1 explains market implied volatility and its relation to the world of derivatives. Second, Section 2.2 describes the random forest machine learning model which is used to perform predictions. Lastly, Section 2.3 describes the hidden Markov model which is used to quantify regimes in market implied volatility.

## 2.1. Market Implied Volatility

In the world of derivatives, options are one of the most prominent types of financial instruments. As sellers of options are exposed to risk for the duration of the contract, they want to be

properly compensated. Measuring this risk requires considering the expected price fluctuations of the underlying asset over the duration of the contract. This expectation is better known as implied volatility and it varies with the strike price and duration of an option contract. To obtain a more general measure, the implied volatility of option contracts that expire on the same date can be combined into a single implied volatility measure. A famous example of this is the CBOE Volatility Index, which combines the implied volatility of di GLYPH&lt;11&gt; erent option contracts on the SPX into an index that is better known as the VIX.

More concretely, the VIX is a measure of expected price fluctuations in the S&amp;P 500 Index over the next 30 days. It is famously known as the fear index and is considered a reflection of investor sentiment on the condition of the market. Equation 1, taken from the VIX white paper (CBOE, 2015), shows how to compute the VIX for a given term T :

<!-- formula-not-decoded -->

where:

T = is time to expiration

F = is the forward index level derived from the index option prices

K 0 = is the first strike below the forward index level F

Ki = is the first strike price of the i th out-of-the-money option: a call if Ki &gt; K 0 , a put if Ki &lt; K 0, or both put and call if Ki = K 0

R = is the risk-free rate to expiration

GLYPH&lt;1&gt; Ki = is the interval between strike prices

Q ( Ki ) = is the midpoint of the bid-ask spread for each option with strike Ki

The equation for computing the VIX is applicable to any asset where option contracts are available. Although this measure can be calculated for any arbitrary term, the duration of the option contracts will seldom match the chosen term T . Indeed, option contracts typically have fixed expiration dates and there is no guarantee that there are option contracts available with a duration equal to the given term. To overcome this obstacle, the VIX is first calculated for the option contracts expiring right before and after the desired target date. The VIX for the given term can then be calculated by linearly interpolating between the two computed measures, as outlined in (CBOE, 2015).

## 2.2. Random Forests

Random forests (Breiman, 2001) are a popular machine learning approach for learning a predictive model. They consist of multiple di GLYPH&lt;11&gt; erent decision (or regression) trees whose predictions are combined into one final prediction. The combination is typically done by taking the mode (or average) of all outputs. While several variations exist for learning a random forest, all of them are relatively straightforward. We summarize one of many popular procedures. Given data D = f ( xi ; yi ) g n i = 1 , where each xi has F features and for k = 1 : : : K trees:

1. Obtain subset d by sampling m &lt; n examples with replacement from D .
2. Train a decision tree on d using a random subset features f GLYPH&lt;18&gt; F , i.e. using CART (Breiman et al., 1984).

The prediction for a regression problem can then be obtained by:

<!-- formula-not-decoded -->

where f is a function in the set of all possible decision trees and K is the total number of trees in the ensemble.

The advantages of random forests include that they are fast to build, are not a GLYPH&lt;11&gt; ected by feature scaling, are robust to irrelevant predictors and noisy data (Khoshgoftaar et al., 2011). Moreover, their method of constructing an ensemble model by randomly subsampling both data points and features during the learning process helps decorrelate the predictions made by the individual trees, which in turn reduces overfitting on the training data.

## 2.3. Hidden Markov Models

Hidden Markov models (HMM) are a generative approach for modeling systems that follow a Markov process (Rabiner and Juang, 1986). The main assumption is that while this process Z is hidden, it can be learned from an observable sequence X whose behaviour depends on Z . More formally, the HMM models the joint distribution of a sequence of hidden states Z and observations X described by:

<!-- formula-not-decoded -->

Given the number of hidden states K and observed sequence X , the model is fully determined by its parameters GLYPH&lt;25&gt; , A , and B which represent the initial state distribution, state transition model, and emission probabilities model, respectively. The initial state distribution is a K GLYPH&lt;2&gt; 1 vector denoting the probabilities that the process is each of the K states in the first timestep. The transition model is a K GLYPH&lt;2&gt; K stochastic matrix where each element Ai ; j denotes the probability of transitioning from state Zt GLYPH&lt;0&gt; 1 ; i to Zt ; j where i ; j 2 f 1 ; : : : ; K g . Lastly, the emissions probability model is a M GLYPH&lt;2&gt; K matrix, with M representing the number of distinct observations, whose elements Bk ; j denote the probability of observing Xt ; k given state Zt ; j .

The three key tasks associated with hidden Markov models are:

1. What is the probability that a sequence of observations X was generated by a given HMM?
2. Given an HMM, what sequence of hidden states Z best explains a given sequence of observations X ?
3. Given a sequence of observations X , learn an HMM with parameters GLYPH&lt;25&gt; , A , and B that would generate them.

The first two tasks can be solved using dynamic programming using the forward-backward algorithm (Chang and Hancock, 1966) and Viterbi (Forney, 1973) algorithm, respectively. The third problem is solved by the Baum-Welch algorithm (Baum, 1972) which uses an iterative expectation-maximization approach.

## 3. Methodology

The main goal of this study is to explore the following questions:

1. To what extent are one-day ahead movements in end-of-day implied volatility predictable, and do features extracted from Twitter improve performance?
2. Does performance vary across the 11 di GLYPH&lt;11&gt; erent stock market sectors? If so, are there any obvious factors that might explain this variability?
3. Can we identify underlying market regimes in implied volatility that influence the performance of our proposed approach?

We tackle the first question by performing an ablation study using random forests on feature configurations including stock price, stock implied volatility, and Twitter features. The study encompasses a universe of 165 stocks over an out-of-sample period spanning January 1st, 2013 till March 1st, 2019. To examine the second question, we diversified our stock universe over the 11 traditional stock sectors and grouped predictive performance by stocks belonging to the same sector. The third and last question was studied by using a hidden Markov model to identify four distinct implied volatility regimes per stock, after which predictive performance was grouped by regime.

The next few sections explain our methodology in more detail. First, Section 3.1 outlines the stock universe we used for our study. Second, Section 3.2 explains how we obtained the relevant data for each stock and how we constructed features for prediction. Section 3.3 and Section 3.4 then respectively show how we used machine learning to predict our target variable and how we evaluated the performance of the approach. Lastly, Section 3.5 explains how we used hidden Markov models to identify regimes in implied volatility which we later use to evaluate our prediction performance through time.

## 3.1. Stock Universe Selection

In order to obtain a diversified universe of stocks, we looked at the popular SPDR and Vanguard Electronic Traded Funds (ETF) that track the 11 traditional US stock market sectors. For each sector, we selected the 15 most liquid stocks based on their average daily dollar-weighted option volume for a total of 165 stocks. Some stocks were excluded due to stock splits (i.e. we kept GOOG and dropped GOOGL), a late introduction to the stock market (i.e. PYPL, ROKU, and SNAP only got introduced after 2015), and ambiguous names making it hard to obtain relevant tweets (i.e. DOW is a chemical company but also a common alias for the Dow Jones Index). Note that we replaced the excluded stocks to maintain 15 stocks per sector for our study. Table 1 provides a concise overview of our stock universe. Refer to Appendix A for a full overview of which stocks were selected per market sector.

## 3.2. Data Acquisition and Feature Generation

We consider data ranging from January 1st, 2011 through March 1st, 2019 for three data sources:

1. Stock price data which consists of historical end-of-day adjusted closing prices for each stock in our universe downloaded from Yahoo Finance.
2. Option contract price data which consists of historical end-of-day option chains for each stock in our universe obtained from IVolatility.
3. Twitter data which consists of all relevant tweets published for each stock. These were collected by filtering on cashtags , which are popular string identifiers authors use to indicate their message is about a certain stock (i.e. a tweet about the Apple stock typically contains $AAPL). In contrast to other research, we did not employ additional filtering

Table 1: This table presents the 11 di GLYPH&lt;11&gt; erent stock market sectors together with their corresponding SPDR ETF symbol and number of stocks considered in this study. The symbols are used to denote sectors throughout this paper, but are not indicative of stocks only belonging to the SPDR ETF portfolio.

| Symbol   | Sector                 |   Selected Stocks |
|----------|------------------------|-------------------|
| XLC      | Communication Services |                15 |
| XLY      | Consumer Discretionary |                15 |
| XLP      | Consumer Staples       |                15 |
| XLE      | Energy                 |                15 |
| XLF      | Financials             |                15 |
| XLV      | Healthcare             |                15 |
| XLB      | Materials              |                15 |
| XLI      | Industrials            |                15 |
| XLK      | Technology             |                15 |
| XLRE     | Real Estate            |                15 |
| XLU      | Utilities              |                15 |

techniques to discard potential spam. Most additional filtering rules appear arbitrary and there seems to be no clear evidence of their validity.

In total, four features were extracted per stock for each trading day. First, we simply used the end-of-day adjusted closing price from the stock price data. Second, we calculated the endof-day 30-day implied volatility using the VIX method on the option contract data. Third and last, we derived two numerical features from our textual Twitter corpus: end-of-day total tweet publication count and end-of-day average sentiment polarity. The former represents the total number of published tweets on a given day. The latter was obtained by performing sentiment analysis using V ADER (Hutto and Gilbert, 2014), a lexicon- and rule-based sentiment model that is specifically well-tailored to social media text, on individual tweets. This yields a sentiment polarity score s 2 [ GLYPH&lt;0&gt; 1 ; 1] for each tweet, which was then used to compute the daily average.

In an e GLYPH&lt;11&gt; ort to capture temporal information residing in the original feature timeseries, we generated two additional predictors per feature. To this end, the daily di GLYPH&lt;11&gt; erence (or first-order di GLYPH&lt;11&gt; erence) and the di GLYPH&lt;11&gt; erence between the daily value and its exponential moving average of the last 10 trading days was taken. Table 2 outlines the di GLYPH&lt;11&gt; erent data sources and their features used in this study. Note that the original adjusted closing price was omitted, as this is typically a non-stationary variable o GLYPH&lt;11&gt; ering little value to a prediction model.

Table 2: This table provides a summary of the features considered per data source. The first row indicates what original features were extracted, whereas the last three rows indicate (*) which features were considered for the actual study. Note that the last two rows denote a specific feature engineering technique applied to the original feature.

| Extracted                | Stocks Adj. Closing Price   | Options Implied Volatility   | Twitter Count, Sentiment   |
|--------------------------|-----------------------------|------------------------------|----------------------------|
| Original                 |                             | *                            | *                          |
| 1st Order Di GLYPH<11> . | *                           | *                            | *                          |
| EMA(10) Di GLYPH<11> .   | *                           | *                            | *                          |

## 3.3. Predicting Movements in Implied Volatility

This study aims to predict one-day ahead movements in a stock's 30-day implied volatility. Concretely, given information at the end of trading day t , we predict whether implied volatility will have moved up or down by the end of next trading day t + 1. To do so, we construct a binary target variable for day t as:

<!-- formula-not-decoded -->

where ivolatility t denotes the end-of-day implied volatility on day t .

In order to predict our target variable, we used random forest classifiers even though more powerful models may exist. For example, the highly popular gradient boosted trees (Friedman, 2001) have been shown to generally perform slightly better than random forests (Caruana et al., 2008; Caruana and Niculescu-Mizil, 2006). However, they are very sensitive to hyper-parameter configurations and require longer runtimes for training. The main goal of this study is not to maximize predictive performance, but rather probe the feasibility of our proposed approach. In addition, it has been suggested that random forests might generally work better on noisy data (Khoshgoftaar et al., 2011), which is especially convenient when working on financial data. Lastly, we did not consider techniques from the domain of deep learning due to the complexity of the models and relatively small number of data points in our study.

Ultimately, we used 64 distinctive random forest configurations built using Sklearn. Each random forest was built with 1000 trees and a unique combination of di GLYPH&lt;11&gt; erent hyper-parameters that control maximum tree depth, the minimum number of samples required to split an internal node, and the minimum number of samples required to be in a leaf node. Each individual tree was built by sampling the training dataset (with replacement) and only considering a random number of p f features where f denotes the total amount of features. The models were trained on a temporally ordered feature matrix X of dimension T GLYPH&lt;2&gt; K , obtained by using any subset of features K from Section 3.2 and period T . Table 3 specifies the possible random forest configurations considered in this study.

Table 3: This table presents the di GLYPH&lt;11&gt; erent possible values considered for di GLYPH&lt;11&gt; erent hyper-parameters available in the random forest implementation of Sklearn. The default value is used for hyper-parameters not listed.

| Hyper-parameter                                                            | Values            |
|----------------------------------------------------------------------------|-------------------|
| n estimators max depth samples samples random state bootstrap max features | f 1000 g          |
|                                                                            | f 4, 6, 8, 10 g   |
| min split                                                                  | f 5, 10, 15, 20 g |
| min leaf                                                                   | f 1, 3, 5, 8 g    |
|                                                                            | f 42 g            |
|                                                                            | yes               |
|                                                                            | sqrt              |

## 3.4. Experimental Evaluation

We evaluated the di GLYPH&lt;11&gt; erent random forest configurations using walk-forward validation, a cross-validation technique designed specifically for temporally ordered data. Classical crossvalidation methods assume observations to be independent. This assumption does not necessarily

Table 4: Example of expanding walk forward validation without where t i represents the feature vector of trading day i . In this example, a training window with an initial size s = 3 is taken together with a testing window of size k = 1. We therefore consistently use the feature vectors of past trading days to train a model (underlined) and subsequently test said model on trading day t + k (bold).

| Iteration   | Variable roles                                                                                              |
|-------------|-------------------------------------------------------------------------------------------------------------|
| 1 2 : : :   | t 1 t 2 t 3 t 4 t 5 GLYPH<1> GLYPH<1> GLYPH<1> t n t 1 t 2 t 3 t 4 t 5 GLYPH<1> GLYPH<1> GLYPH<1> t n : : : |
| m           | t 1 GLYPH<1> GLYPH<1> GLYPH<1> t n GLYPH<0> 3 t n GLYPH<0> 2 t n GLYPH<0> 1 t n                             |

hold for timeseries data, which inherently contains temporal dependencies among observations. To this end, the dataset is repeatedly split up in training and test sets where temporal order is accounted for. In our case, we used an expanding window of initially 504 trading days to train the models, after which performance was measured on the next out-of-sample 40 trading days. Table 5 shows an example of this method where t i denotes the feature vector corresponding to trading day i . Note that in this scenario, when given a total of n observations, an expanding training window of length t and an out-of-sample test window of length k , you can construct a maximum of n GLYPH&lt;0&gt; t GLYPH&lt;0&gt; k di GLYPH&lt;11&gt; erent train-test splits. Ultimately, each configuration its performance is averaged across all folds. We measured performance with the area under the receiver operating characteristic curve metric (AUC hereafter).

Table 5: Example of walk-forward validation where t i represents the feature vector of trading day i . In this example, a training window with an initial size s = 3 is taken together with a testing window of size k = 1. We therefore consistently use the feature vectors of past trading days to train a model (underlined) and subsequently test said model on trading day t + k (bold).

| Iteration   | Variable roles                                                                                              |
|-------------|-------------------------------------------------------------------------------------------------------------|
| 1 2 : : :   | t 1 t 2 t 3 t 4 t 5 GLYPH<1> GLYPH<1> GLYPH<1> t n t 1 t 2 t 3 t 4 t 5 GLYPH<1> GLYPH<1> GLYPH<1> t n : : : |
| m           | t 1 GLYPH<1> GLYPH<1> GLYPH<1> t n GLYPH<0> 3 t n GLYPH<0> 2 t n GLYPH<0> 1 t n                             |

## 3.5. Analyzing Performance through Time with Hidden Markov Models

The ever-changing nature of financial markets makes it hard to find approaches that consistently work well. The ability to time approaches therefore becomes an interesting perk. In an e GLYPH&lt;11&gt; ort to evaluate how our proposed approach weathers the evolution of the financial market through time, we look at its performance under di GLYPH&lt;11&gt; erent market regimes.

We quantified market regimes as di GLYPH&lt;11&gt; erent states in mean implied volatility using a hidden Markov model. For each stock, a di GLYPH&lt;11&gt; erent HMM model was trained on its end-of-day implied volatility timeseries dating from January 1st, 2007 till December 31st, 2012 and was used out-of-sample thereafter. Analogous to a study performed by Soci´ et´ e G´ en´ erale (Daviaud et al., 2020), four di GLYPH&lt;11&gt; erent regimes were identified corresponding to low, medium, high, and very high mean implied volatility. Table 6 specifies the hyper-parameter configuration used to build hidden Markov models using the hmmlearn Python package.

Table 6: This table presents the hyper-parameter configuration used for building hidden Markov models using the hmmlearn package. The default value is used for hyper-parameters not listed.

| Hyper-parameter   | Values   |
|-------------------|----------|
| n components      | 4        |
| n iter            | 100      |
| random state      | 42       |
| emissions         | Gaussian |
| algorithm         | Viterbi  |

## 4. Experimental Results and Discussion

In this section, we present and discuss our experimental results. First, Section 4.1 shows the results of our ablation study where in total seven di GLYPH&lt;11&gt; erent feature configurations were considered. Section 4.2 then builds on these results by looking at the performance of the best feature configuration per market sector. Lastly, Section 4.3 looks at predictive performance across different implied volatility regimes. Recall that throughout the remainder of this section we may denote the 11 di GLYPH&lt;11&gt; erent stock market sectors by the symbol of their equivalent SPDR ETF tracker. This is solely done out of convenience and is not indicative of stocks only belonging to said ETF portfolio. Refer to Table 1 for an overview of the sector symbols.

## 4.1. Ablation Study

The first objective of this study was to investigate to what extent daily movements in endof-day implied volatility can be predicted. To this end, we obtained 11 di GLYPH&lt;11&gt; erent features from three di GLYPH&lt;11&gt; erent data sources (Section 3.2) on which we built random forest classifiers to predict said target variable. We assessed the e GLYPH&lt;11&gt; ectiveness of the di GLYPH&lt;11&gt; erent data sources by performing an ablation study where 7 di GLYPH&lt;11&gt; erent scenarios were considered, shown in Table 7. In total, one-day ahead movements in implied volatility were predicted for 165 stocks (Section 3.1) spanning an out-of-sample period from January 1st, 2013 till March 1st, 2019.

Table 7: This tables shows the di GLYPH&lt;11&gt; erent feature scenarios considered in our ablation study together with their total number of features. Note that the third column indicates the usage of both original and derived features from the given feature source.

|   Scenario | Feature Source                          |   Features |
|------------|-----------------------------------------|------------|
|          1 | Stock Price                             |          2 |
|          2 | Stock Price, Tweets                     |          8 |
|          3 | Implied Volatility                      |          3 |
|          4 | Implied Volatility, Tweets              |          9 |
|          5 | Tweets                                  |          6 |
|          6 | Stock Price, Implied Volatility         |          5 |
|          7 | Stock Price, Implied Volatility, Tweets |         11 |

We compared the predictive performance of di GLYPH&lt;11&gt; erent scenarios to that of a stratified dummy classifier, which makes the comparison more rigid than using a simple random classifier. Indeed, implied volatility tends to go down more often than it goes up. This causes a stratified dummy

classifier to achieve a median AUC of 51.8% across all 165 stocks versus a 50.0% achieved by a fully random one.

Table 8 displays the median AUC achieved for each scenario averaged over the entire selected stock universe and the di GLYPH&lt;11&gt; erence in AUC between our approach and the stratified dummy classifier. These results provide empirical evidence that end-of-day movements in implied volatility can indeed be predicted. All possible feature scenarios perform better than a purely random classifier that achieves a median of 50.0% AUC. Moreover, 4 out of 7 scenarios outperform the stratified dummy classifier that achieves a median of 51.8% AUC. The commonality among these improved scenarios is the use of implied volatility features, indicating that this is an important source of information. Moreover, including features derived from tweets always yielded a better median performance (S2 versus S1, S4 versus S3, and S7 versus S6). This implies there is indeed a predictive interplay between information on Twitter and future implied volatility. Lastly, using all possible features (S7) yielded the best result overall, suggesting there are predictive patterns among all three feature sources.

Table 8: This table displays the median predictive performance across all 165 stocks per feature configuration obtained by predicting daily end-of-day movements in implied volatility over the period of January 1st, 2013 to March 1st, 2019. Moreover, the second row shows how much the proposed approach does better than the stratified dummy classifier.

|             |   S1 |   S2 | S3    | S4    |   S5 | S6    | S7    |
|-------------|------|------|-------|-------|------|-------|-------|
| Median AUC  | 51.1 | 51.6 | 53.6  | 54.2  | 50.9 | 54.3  | 55.1  |
| Improvement | -0.6 | -0.1 | + 1.9 | + 2.5 | -0.8 | + 2.7 | + 3.4 |

## 4.2. Predictive Performance across Sectors

In this section we look at the best performing feature configuration (S7) and its performance variability across 11 di GLYPH&lt;11&gt; erent stock market sectors. Figure 1 shows a box plot where the performance improvement of our proposed approach versus the stratified dummy classifier for each individual stock is grouped by sector. The results were obtained on an out-of-sample period spanning January 1st, 2013 till March 1st, 2019.

It is clear that our proposed methodology is generally able to beat the stratified dummy classifier across all di GLYPH&lt;11&gt; erent sectors. The approach beats the dummy classifier on 148 out of 165 stocks. However, there is a considerable amount of variability present across di GLYPH&lt;11&gt; erent sectors. Indeed, the approach does significantly better on XLRE and XLU, but predictions on XLC, XLY, and XLK also do better comparatively. In contrast, XLI and XLB seem to lack in performance. The next two subsections will showcase a preliminary attempt to partially explain this variability in performance.

## 4.2.1. The E GLYPH&lt;11&gt; ect of Option Liquidity

The results from the previous section indicate that predictions on stocks from both XLRE and XLU do significantly better. Remarkably, it turns out that stocks in these two sectors are also significantly less liquid compared to other sectors. Figure 2 respectively shows a box plot of median option liquidity per sector, measured by the average daily dollar amount traded in options, and a regression plot where the relationship between liquidity and performance improvement versus the dummy classifier is outlined.

These results seem to suggest that there is indeed a weak negative correlation between predictive performance and option liquidity, implying that less liquid stocks are easier to predict.

Figure 1: This box plot shows the performance improvement of our proposed methodology using feature configuration S7 versus a stratified dummy classifier on individual stocks grouped by sector. The red dotted line represents the minimum threshold necessary to beat the stratified dummy classifier. The blue dotted line represents the overall median improvement of our proposed approach versus the stratified dummy classifier.

<!-- image -->

Figure 2: This figure presents a box plot (left) where the median of daily stock option liquidity is grouped per sector and a regression plot (right) where the relationship between stock option liquidity and performance improvement versus the dummy classifier is outlined.

<!-- image -->

We hypothesize that the relatively undersized liquidity of both XLRE and XLU in the options market is possibly accompanied by a less e GLYPH&lt;14&gt; cient price discovery process. This in turn might cause these markets to reflect new information more slowly, making them easier to predict with the information at hand. However, we note that is only one possible explanation and many other factors might lie at the basis of this phenomenon.

## 4.2.2. The E GLYPH&lt;11&gt; ect of Twitter Attention

Lower liquidity might partially explain why sectors such as XLRE and XLU seem easier to predict, but it certainly does not tell the whole story. Indeed, predictions on stocks from XLC, XLY, and XLK, examples of very liquid sectors, also do better comparatively. Here, we hypothesize that this might be due to the attention they receive on Twitter. Figure 3 respectively shows a box plot of the median daily tweets published on stocks grouped per sector and a regression plot where the relationship between liquidity and daily tweets is outlined.

Figure 3: This figure presents a box plot (left) where the median of daily tweet publication of stocks is grouped per sector and a regression plot (right) where the relationship between a stock its daily tweets and option liquidity is outlined.

<!-- image -->

The box plot on the left of Figure 3 shows that stocks in XLC, XLY, and XLK receive significantly more attention on Twitter than others. Note that the plot demonstrates a striking resemblance with the box plot showing liquidity per sector in Figure 2. Indeed, the regression plot on the right shows a very strong correlation between attention on Twitter and liquidity. These findings seem to suggest that prediction is easier on stocks that are more popular on social media. We investigated this phenomenon further by looking at the improvement in predictive performance caused by features extracted from Twitter per sector. More concrete, we looked at the di GLYPH&lt;11&gt; erence in performance between feature configuration S6, which uses stock and options features, and S7, which combines features from S6 and Twitter features (Section 4.1). Figure 4 shows the median improvement of using Twitter features per stock grouped by sector.

With exception of XLE, most sectors seem to have a sizeable number of stocks that benefit from using social media features. This is no surprise considering the results presented in Section 4.1. However, it seems that sectors that receive more social media attention seem to benefit the most. For example, the three sectors XLC, XLY, and XLK that are most popular also benefit more consistently followed closely by XLV. Two reasons might explain these results. First, previous research has hinted at social media inciting herd behavior and emotional reactions among investors, possibly driving ine GLYPH&lt;14&gt; ciency (e.g. Wang and Wang, 2018; Oliveira et al., 2017; GroßKlußmann et al., 2019). If this is true, it makes sense that Twitter features provide more added value for popular stocks. Second, our sentiment extraction technique on tweets is imperfect and might impact these results as well. Without carefully filtering out tweets that are advertisements or spam, we rely on the law of large numbers to correctly estimate a stock its sentiment for a

Figure 4: This figure shows the performance improvement of using Twitter features together with features from S6, versus only using features from S6. The red dotted line represents the minimum threshold necessary to beat the feature configuration of S6.

<!-- image -->

given day. Naturally, daily sentiment estimation will therefore be better on stocks that are more heavily tweeted about.

## 4.3. Performance across Implied Volatility Regimes

In this section, we look at the best-performing feature configuration (S7) and its performance variability across four di GLYPH&lt;11&gt; erent market regimes in implied volatility, identified by using a hidden Markov model. Table 9 shows the median of all results across 165 stocks where, for each of the four regimes, the columns respectively show how many days were spent in a regime, what the average implied volatility was, the AUC of a stratified dummy classifier, and how much better our approach did compared to the latter.

Table 9: This table shows the median number of days a stock resided in one out of four implied volatility regimes, together with each regime's mean implied volatility, stratified dummy performance and the improvement of our approach. In total, 165 stocks were considered over a period spanning January 1st, 2013 till March 1st, 2019.

|           |   Frequency |   Implied Volatility |   Dummy AUC | Improvement   |
|-----------|-------------|----------------------|-------------|---------------|
| Low       |         392 |                 18.6 |       50.75 | + 3.15        |
| Medium    |         452 |                 22.3 |       50.65 | + 3.83        |
| High      |         411 |                 26.7 |       52.12 | + 3.84        |
| Very High |         231 |                 35.3 |       54.92 | + 1.47        |

In general, stocks seem to reside more in the lower implied volatility environments. The stratified dummy classifier also appears to perform worse here, implying that up and down movements are about equal in occurrence. However, the dummy classifier performance picks up as implied volatility increases. This makes sense from a theoretical perspective, as implied volatility is deemed to be a mean reverting process characterized by big up movements after which the variable slowly trails back down, resulting in more down movements. Although our approach

seems to outperform the dummy classifier in all regimes, the added value seems to be most significant in the low to high regimes. This may suggest that distressed markets are harder to predict than their calmer counterparts. However, note that the improvement of our approach seems to be slightly correlated with regime frequency. From a data science perspective, this might imply that the model performs worse in less frequent regimes because it simply had fewer examples to learn from. Lastly, Table 10 o GLYPH&lt;11&gt; ers a more finer-grained analysis where the median improvement for each regime per sector is shown.

Table 10: This table shows the median improvement of our approach compared to a stratified dummy classifier for stocks grouped per market sector and implied volatility regime. The best and worst regime for each sector are respectively indicated by bold and underlined text styles.

|      |   Low |   Medium |   High |   Very High |
|------|-------|----------|--------|-------------|
| XLB  |  1.87 |     3.06 |   1.02 |        0.77 |
| XLC  |  3.57 |     4.76 |   5.27 |        0.26 |
| XLE  |  2.55 |     2.46 |   4    |       -2.55 |
| XLF  |  3.06 |     4.08 |   2.64 |       -1.7  |
| XLI  |  0.05 |     1.53 |   0.17 |       -1.5  |
| XLK  |  0.6  |     2.38 |   3.91 |        5.7  |
| XLP  |  2.72 |     3.83 |   3.23 |        1.53 |
| XLRE |  8.5  |     6.38 |   4.93 |        4.25 |
| XLU  |  6.63 |     5.95 |   5.36 |        1.79 |
| XLV  |  4.25 |     3.23 |   1.62 |        2.13 |
| XLY  |  2.04 |     3.66 |   5.95 |        0.85 |

Again, we remark a significant amount of variability in performance across both regimes and sectors. Optimal implied volatility regimes seem to di GLYPH&lt;11&gt; er significantly for di GLYPH&lt;11&gt; erent sectors. In contrast, with exception of XLK, sectors seem to comparatively do worse when implied volatility is very high. As all sectors share roughly the same regime frequency distribution, no clear reason emerges to explain the performance variability. One potential reason might be due to sector idiosyncrasies. For example, defensive sectors such as XLRE, XLU, and XLV seem to have lower optimal regimes than cyclical sectors such as XLC, XLK, and XLY. However, this is not always the case. Lastly, we have no explanation for the significant di GLYPH&lt;11&gt; erence between XLK and the other sectors. Here, our approach performs best in the highest implied volatility regime and worst in the lowest. This is in stark contrast with the other sectors where the opposite is true. Perhaps the speculative nature of technology stocks is more sensitive to herd behaviour and therefore investor irrationality.

## 5. Conclusion

In this study, we presented the first empirical evidence that one-day ahead movements of end-of-day stock implied volatility can be predicted to a certain extent, and that attention and sentiment features extracted from Twitter improve the performance of the approach. These alternative features were not able to predict implied volatility in isolation, but improved the approach significantly when combined with predictors extracted from stock and options data. This suggests that the interplay between these sources gives rise to predictive patterns. By conducting

our experiments on a diversified universe of 165 US stocks, we were able to assess the predictive performance across 11 traditional stock market sectors and found that stocks in real estate, utilities, consumer discretionary, communications, and technology were easier to predict than others. Further analysis indicated that these di GLYPH&lt;11&gt; erences could potentially be explained by market ine GLYPH&lt;14&gt; ciencies caused by low option liquidity in the real estate and utilities sector, and excessive Twitter attention in the consumer discretionary, communications, and technology sector. Lastly, using hidden Markov models, we evaluated the predictive performance of our approach across four di GLYPH&lt;11&gt; erent implied volatility regimes. Although it outperforms the dummy classifier in all four regimes, we found that it yields the least improvement in the regime associated with the highest average implied volatility. Moreover, we discovered that di GLYPH&lt;11&gt; erent stock market sectors have di GLYPH&lt;11&gt; erent optimal regimes for the application of our approach. By analyzing performance through the usage of regimes, we showed that this alternative quantification of time provides additional insight into the performance of models which in turn could help better anticipate their future performance.

## Appendix A. Selected Stock Universe

This appendix contains more detailed information on the US stock universe used in this study. Table A.11 outlines which stocks were chosen for each traditional market sector.

Table A.11: This table presents the US stock universe that was used in this study. Note that for each traditional market sector we chose 15 stocks based on option liquidity.

| Materials              | FCX DD LYB   | X IP VMC   | NEM CF SHW   | CLF AA BLL   | MOS NUE WRK   |
|------------------------|--------------|------------|--------------|--------------|---------------|
|                        | FB           | NFLX       | GOOG         | T            | TWTR          |
| Communications         | VZ           | DIS        | CMCSA        | EA           | YELP          |
|                        | ATVI         |            |              | Z            | TTWO          |
|                        |              | DISH       | CHTR         | COP          |               |
| Financials Industrials | XOM HAL      | OXY VLO    | CVX EOG      | APA          | SLB KMI       |
| Energy                 | HES          |            | MRO          | RIG          |               |
|                        |              | MPC        |              |              | WMB           |
|                        | BAC AIG      | JPM BX     | C MS         | GS AXP       | WFC           |
|                        |              |            |              |              | CME           |
|                        | MET          | USB        | SCHW         | COF          | BLK           |
|                        | GE           | BA         | CAT          | LMT          | UPS           |
|                        | UNP CSX      | FDX NSC    | DE EMR       | MMM NOC      | HON ETN       |
|                        |              |            |              | INTC         | MU            |
| Technology             | AAPL         | NVDA       | MSFT         |              |               |
|                        | IBM          | QCOM       | CSCO         | CRM          | AMD           |
|                        | MA           | V          | ORCL         | ADBE         | TXN MO        |
| C. Staples             | PG           | WMT        | PM           | KO           |               |
|                        | CL           | COST       | PEP          | HLF CAG      | WBA           |
|                        | GIS          | MNST       | TSN          | EQIX         | KR            |
| Real Estate            | SPG          | WY         | AMT          |              | IRM           |
|                        | CCI          | PSA        | AVB          | VTR          | O             |
|                        | HST          | DLR        | PLD          | MAC          | EQR           |
|                        | SO           | EXC        | AEP          | DUK          | NRG           |
| Utilities              | NEE          | FE         | D            | PPL          | ED            |
|                        | ETR          | EIX        | CNP          | NI           | SRE           |
|                        | PFE          | JNJ        | GILD         | LLY          | MRK           |
| Healthcare             | ABT          | BMY        | AMGN         | UNH          | CVS           |
|                        | ABBV         | ISRG       | MDT          | CI           | DHR           |
|                        | AMZN         | TSLA       | MCD          | HD           | F             |
| C. Discretionary       | CMG TGT      | GM LOW     | SBUX BBY     | EBAY LULU    | NKE MGM       |

## References

- Baker, M., Wurgler, J., 2006. Investor sentiment and the cross-section of stock returns. The Journal of Finance 61, 1645-1680. doi: https://doi.org/10.1111/j.1540-6261.2006.00885.x .
- Baum, L.E., 1972. An inequality and associated maximization technique in statistical estimation for probabilistic functions of Markov processes, in: Inequalities III: Proceedings of the Third Symposium on Inequalities, Academic Press. pp. 1-8.
- Bollen, J., Mao, H., Zeng, X., 2011. The impact of social and conventional media on firm equity value: A sentiment analysis approach. Journal of Computational Science 2, 1-8. doi: https://doi.org/10.1016/j.jocs.2010.12. 007 .
- Breiman, L., 2001. Random forests. Mach. Learn. 45, 5-32. doi: https://doi.org/10.1023/A:1010933404324 .
- Breiman, L., Friedman, J.H., Olshen, R.A., Stone, C.J., 1984. Classification and Regression Trees. Wadsworth and Brooks, Monterey, CA. doi: https://doi.org/10.1201/9781315139470 .
- Caruana, R., Karampatziakis, N., Yessenalina, A., 2008. An empirical evaluation of supervised learning in high dimensions, in: International Conference on Machine Learning (ICML), pp. 96-103. doi: https://doi.org/10.1145/ 1390156.1390169 .
- Caruana, R., Niculescu-Mizil, A., 2006. An empirical comparison of supervised learning algorithms, in: Proceedings of the 23rd International Conference on Machine Learning, Association for Computing Machinery, New York, NY, USA. p. 161-168. doi: https://doi.org/10.1145/1143844.1143865 .
- CBOE, 2015. Cboe volatility index white paper URL: https://www.cboe.com/micro/vix/vixwhite.pdf .
- Chang, R., Hancock, J., 1966. On receiver structures for channels having memory. IEEE Transactions on Information Theory 12, 463-468. doi: https://doi.org/10.1109/TIT.1966.1053923 .
- Chiang, T.C., Zheng, D., 2010. An empirical analysis of herd behavior in global stock markets. Journal of Banking &amp; Finance 34, 1911-1921. doi: https://doi.org/10.1016/j.jbankfin.2009.12.014 .
- Daviaud, O., Korber, O., Mukhopadhyay, A., Ungari, S., 2020. Systematic trading in options. Soci´ et´ e G´ en´ erale - Cross Asset Research .
- Fama, E.F., 1970. E GLYPH&lt;14&gt; cient capital markets: A review of theory and empirical work. The Journal of Finance 25, 383-417. doi: https://doi.org/10.2307/2325486 .
- Forney, G.D., 1973. The viterbi algorithm. Proceedings of the IEEE 61, 268-278. doi: https://doi.org/10.1109/ PROC.1973.9030 .
- Friedman, J.H., 2001. Greedy function approximation: A gradient boosting machine. The Annals of Statistics 29, 1189 - 1232. doi: https://doi.org/10.1214/aos/1013203451 .
- Groß-Klußmann, A., K¨ onig, S., Ebner, M., 2019. Buzzwords build momentum: Global financial twitter sentiment and the aggregate stock market. Expert Systems with Applications 136, 171-186. doi: https://doi.org/10.1016/j. eswa.2019.06.027 .
- Guijarro, F., Moya-Clemente, I., Saleemi, J., 2019. Liquidity Risk and Investors' Mood: Linking the Financial Market Liquidity to Sentiment Analysis through Twitter in the S&amp;P500 Index. Sustainability 11, 1-13. doi: https://doi. org/10.3390/su11247048 .
- Hutto, C.J., Gilbert, E., 2014. Vader: A parsimonious rule-based model for sentiment analysis of social media text., in: Adar, E., Resnick, P., Choudhury, M.D., Hogan, B., Oh, A.H. (Eds.), ICWSM, The AAAI Press.
- Karagozoglu, A.K., Fabozzi, F.J., 2017. Volatility wisdom of social media crowds. The Journal of Portfolio Management 43, 136-151. doi: https://doi.org/10.3905/jpm.2017.43.2.136 .
- Khoshgoftaar, T.M., Van Hulse, J., Napolitano, A., 2011. Comparing boosting and bagging techniques with noisy and imbalanced data. IEEE Transactions on Systems, Man, and Cybernetics - Part A: Systems and Humans 41, 552-568. doi: https://doi.org/10.1109/TSMCA.2010.2084081 .
- Li, T., van Dalen, J., van Rees, P.J., 2018. More than just noise? examining the information content of stock microblogs on financial markets. Journal of Information Technology 33, 50-69. doi: https://doi.org/10.1057/ s41265-016-0034-2 .
- Oliveira, N., Cortez, P., Areal, N., 2017. The impact of microblogging data for stock market prediction: using twitter to predict returns, volatility, trading volume and survey sentiment indices. Expert systems with applications 73, 125-144. doi: https://doi.org/10.1016/j.eswa.2016.12.036 .
- Rabiner, L., Juang, B., 1986. An introduction to hidden markov models. IEEE ASSP Magazine 3, 4-16. doi: https: //doi.org/10.1109/MASSP.1986.1165342 .
- Schnaubelt, M., Fischer, T.G., Krauss, C., 2020. Separating the signal from the noise - financial machine learning for twitter. Journal of Economic Dynamics and Control 114, 103895. doi: https://doi.org/10.1016/j.jedc. 2020.103895 .
- Wang, G., Wang, Y., 2018. Herding, social network and volatility. Economic Modelling 68, 74-81. doi: https: //doi.org/10.1016/j.econmod.2017.04.018 .