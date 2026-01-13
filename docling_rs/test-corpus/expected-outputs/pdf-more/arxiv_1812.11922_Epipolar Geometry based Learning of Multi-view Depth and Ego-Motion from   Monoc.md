## Epipolar Geometry based Learning of Multi-view Depth and Ego-Motion from Monocular Sequences

Vignesh Prasad, Dipanjan Das, Brojeshwar Bhowmick

Embedded Systems &amp; Robotics, TCS Research &amp; Innovation, Kolkata

{vignesh.prasad,dipanjan.da,b.bhowmick}@tcs.com and Guido Gerig (Eds.). ACM, New York, NY, USA, 10 pages. https://doi.org/ 10.1145/3293353.3293427

## 1 INTRODUCTION

In recent years, there has been an increasing trend in using deep networks for predicting dense depth and ego-motion from monocular sequences. Such methods, including the one proposed in this paper, make inferences of the scene by observing a lot of samples and inferring their understanding based on consistencies in the scene, similar to the way humans do. Building on top of the same idea of ensuring photometric consistency, some methods add additional supervised constraints in the form of ground-truth depth [5, 6, 20, 37], calibrated stereo rigs [12, 16] or ground-truth poses[36] or both pose and depth[39]. SE3-Nets[4] take it a step further and operate directly on pointcloud data to estimate rigid body motions.

SfMLearner[47] was one of the major developments in this field which popularized the ability of deep networks for the task at hand. They were the first method to do it in a purely monocular manner. In order to deal with non-rigid objects (cars, pedestrians etc.), they predict an "explainability mask" in order to discount regions that violate the static scene assumption. GeoNet[46] and SfM-Net[40] tackle this issue by explicitly predicting object motions and incorporating optical.

However, one fallacy in these methods is that rather than taking a geometric approach to the problem, they play around with losses to get better performance. We build upon the SfMLearner pipeline and enrich it with the fundamentals of computer vision and multiview geometry. We tackle the problem in a more geometrically meaningful way by constraining the correspondences to lie on their corresponding epipolar lines. This is done by weighting the losses using epipolar constraints with the Essential Matrix obtained from Nistér's Five Point Algorithm [34]. This helps us account for violations of the static scene assumption, and also to tackle the problem of improper correspondence generation that arises by minimizing just the photometric loss. We make use of the Five Point Algorithm to help guide the training and improve the predictions. Moreover, rather than inferring depth from a single view, we try to learn inter-pixel relationships by predicting depth from two views in a sequence. Note that we do not use stereo pairs that have a wide baseline, but a sequence of monocular views.

Our main contributions are twofold. The first is incorporating a 2-view depth prediction, rather than from a single view. One thing to note is that, by 2-view we mean two consecutive frames and not a pair of stereo images. Incorporating this helps improve the depth prediction, which is shown in Sec. 5.3. Secondly, we incorporate epipolar constraints to make the learning more geometrically oriented. We do so by using the per-pixel epipolar distance as a weighting factor to help deal with occlusions and non-rigid objects.

## ABSTRACT

Deep approaches to predict monocular depth and ego-motion have grown in recent years due to their ability to produce dense depth from monocular images. The main idea behind them is to optimize the photometric consistency over image sequences by warping one view into another, similar to direct visual odometry methods. One major drawback is that these methods infer depth from a single view, which might not effectively capture the relation between pixels. Moreover, simply minimizing the photometric loss does not ensure proper pixel correspondences, which is a key factor for accurate depth and pose estimations.

In contrast, we propose a 2-view depth network to infer the scene depth from consecutive frames, thereby learning inter-pixel relationships. To ensure better correspondences, thereby better geometric understanding, we propose incorporating epipolar constraints to make the learning more geometrically sound. We use the Essential matrix obtained using Nistér's Five Point Algorithm, to enforce meaningful geometric constraints, rather than using it as training labels. This allows us to use lesser no. of trainable parameters compared to state-of-the-art methods. The proposed method results in better depth images and pose estimates, which capture the scene structure and motion in a better way. Such a geometrically constrained learning performs successfully even in cases where simply minimizing the photometric error would fail.

## CCS CONCEPTS

· Computing methodologies Epipolar geometry ; Computer vision ; Vision for robotics ; 3D imaging ; Reconstruction ; Unsupervised learning ; Neural networks.

## KEYWORDS

Monocular Depth, Visual Odometry, Epipolar Geometry, Computer Vision, Unsupervised Learning

## ACMReference Format:

Vignesh Prasad, Dipanjan Das, Brojeshwar Bhowmick. 2018. Epipolar Geometry based Learning of Multi-view Depth and Ego-Motion from Monocular Sequences. In 11th Indian Conference on Computer Vision, Graphics and Image Processing (ICVGIP 2018), December 18-22, 2018, Hyderabad, India, Anoop M. Namboodiri, Vineeth Balasubramanian, Amit Roy-Chowdhury,

Permission to make digital or hard copies of all or part of this work for personal or classroom use is granted without fee provided that copies are not made or distributed for profit or commercial advantage and that copies bear this notice and the full citation on the first page. Copyrights for components of this work owned by others than ACM must be honored. Abstracting with credit is permitted. To copy otherwise, or republish, to post on servers or to redistribute to lists, requires prior specific permission and/or a fee. Request permissions from permissions@acm.org.

ICVGIP 2018, December 18-22 2018, Hyderabad, India

© 2018 Association for Computing Machinery.

ACM ISBN 978-1-4503-6615-1/18/12...$15.00 https://doi.org/10.1145/3293353.3293427

This not only enforces pixel level correspondences but also allows us to do away with the explainability mask thereby having lesser number of parameters to predict.

## 2 BACKGROUND

## 2.1 Structure-from-Motion (SfM)

Structure-from-Motion refers to the task of reconstructing an environment and recovering the camera motion from a sequence of images. Various methods exist that aim to tackle the problem as it is an age-old problem in vision[3, 11, 38, 44] however, they usually require computation of accurate point correspondences for performing the task. A subset of this comes under the area of monocular SLAM, which involves solving the SfM problem in realtime. Visual odometry, is a smaller subset which doesn't involve structural estimation, but just camera motion estimation. These approaches could either be sparse[10, 25, 27, 30, 32], semi-dense[7, 8] or dense[2, 33]. The main issue that arises in these methods is that of improper correspondences in texture-less areas, or if there are occlusions or repeating patterns. In monocular approaches, performing it in a sparse manner is a well known topic but estimating dense monocular depth is much more complex.

## 2.2 Depth Image Based Warping

In order to ensure that a reconstruction is accurate, one way is to ensure that the reprojected image of the predicted scene at a novel view point is consistent with what is observed at that point. This consistency is known as photometric consistency. One common approach for image warping in the context of deep networks using differentiable warping and bi-linear sampling [22] which have been in use for a variety of applications like learning optical flow [23], video prediction [35] and image captioning [24]. We apply bi-linear sampling to calculate the warped image using scene depth and the relative transformation between two views.

In this approach, given a warped pixel ˆ p , its pixel value I s ( ˆ p ) is interpolated using 4 nearby pixel values of ˆ p (upper left and right, lower left and right) i.e. ˆ I s ( p ) = I s ( ˆ p ) = ˝ i ˝ j w i j I s ( p i j ) where i ∈ {⌊ ˆ p x ⌋ , ⌈ ˆ p x ⌉} , j ∈ {⌊ ˆ p y ⌋ , ⌈ ˆ p y ⌉} and w i j is directly proportional to the proximity between ˆ p and p i j such that ˝ w i j = 1. Further explanation regarding this can be found in [22].

## 2.3 Epipolar Geometry

We know that a pixel p in an image corresponds to a ray in 3D, which is given by its normalized coordinates ˜ p = K -1 p , where K is the intrinsic calibration matrix of the camera. From a second view, the image of the first camera center is called the epipole and that of a ray is called the epipolar line. Given the corresponding pixel in the second view, it should lie on the corresponding epipolar line. This constraint on the pixels can be expressed using the Essential Matrix E for calibrated cameras. The Essential Matrix contains information about the relative poses between the views. Detailed information regarding normalized coordinates, epipolar geometry, Essential Matrix etc, can be found in [19]. Given a pixel's normalized coordinates ˜ p and that of its corresponding pixel in a second view ˆ ˜ p , the relation between ˜ p , ˆ ˜ p and E can be expressed as:

<!-- formula-not-decoded -->

Here, E ˜ p is the epipolar line in the second view corresponding to a pixel p in the first view. In most cases, there could be errors in capturing the pixel p , finding the corresponding pixel ˆ p or in estimating the Essential Matrix E . Therefore in most real world applications, rather than imposing Eq. 1, the value of ˆ ˜ p T E ˜ p is minimized in a RANSAC[9] like setting. We refer to this value as the epipolar loss in the rest of the paper.

We refer to a pixel's homogeneous coordinates as p and the normalized coordinates as ˜ p in the rest of the paper. We also refer to a corresponding pixel in a different view as ˆ p and in normalized coordinates as ˆ ˜ p .

## 2.4 Nistér's 5 Point Algorithm

Nistér's 5 Point Algorithm [34] is currently the state-of-the-art solutions to the "Five Point Relative Pose" problem for estimating the Essential Matrix between two views. The problem can be stated as follows. Given the projections of five unknown points onto two unknown views, what are the possible configurations of points and cameras? The solution to this gives rise to the relative poses between the cameras and the points. Absolute scale, however, cannot be recovered from just images. Nistér proposes a solution by solving a tenth degree polynomial in order to extract the Essential Matrix E which can then be decomposed into the rotation R and translation t between the two views.

## 3 PROPOSED APPROACH

We use 2 CNNs, one for inverse depth (which we refer to as depth network) and one for pose (pose network). Our Depth network takes a pair of consecutive images as input, rather than a single image, and calculates the depth of the scene as seen in the first image. The reason behind this is to leverage the relationship between pixels over multiple views to calculate the depth, rather than relying on learnt semantics of the scene in a single view. The Pose network takes an image sequence as input. The first image is the target view with respect to which the poses of the other images are calculated. Both networks are independent of each other and are trained jointly to effectively capture the coupling between scene depth and camera motion in learning based paradigm.

The main idea behind the training, similar to that of previous works, is to ensure proper scene reconstruction between the source and target views based on the predicted depth and poses. We warp the source view into the target frame and minimize the photometric error between the synthesized image and the target image to ensure that the predicted depth and pose can recreate the scene effectively. Details about this warping process is given in Sec. 3.1 and 3.2. Additionally, since we predict the depth using each source image as the second input image, we need to ensure consistency between the predicted depth images. This is explained in Sec. 3.3.

For now, we consider mostly static scenes, i.e. where objects in the scene are rigid. SfMLearner predicts an "explainability mask" along with the pose, which denotes the contribution of a pixel to the loss such that pixels of non-rigid objects have a low weight. Instead, we use the epipolar loss to weight the pixels. This process is explained in Sec. 3.5.

Depth CNN

Epipolar Geometry based Learning of Multi-view Depth and Ego-Motion from Monocular Sequences

Error Images

ICVGIP 2018, December 18-22 2018, Hyderabad, India

Figure 1: Overview of the training procedure. The Depth CNN predicts the inverse depth for a target view by taking in the target view and a nearby image as the input. The Pose CNN predicts the relative poses of the source views from the target, which are then warped into the target frame using the relative poses and the scene depth and the photometric errors between multiple source-target frame pairs are minimized. These are weighted by the per-pixel epipolar loss.

<!-- image -->

## 3.1 Image Warping

Given a pixel p in normalized coordinates, and its depth D ( p ) , we transform it into the source frame using the relative pose and project it onto the source image's plane.

<!-- formula-not-decoded -->

where K is the camera calibration matrix, D ( p ) is the depth of pixel p , R t → s and t t → s are the rotation and translation respectively from the target frame to the source frame. The homogeneous coordinates of ˆ p are continuous while we require integer values. Thus, we interpolate the values from nearby pixels, using bi-linear sampling, proposed by [22], as explained in sec 2.2.

## 3.2 Novel View Synthesis

We use novel view synthesis using depth image based warping as the main supervisory signal. Given the per-pixel depth and the relative pose between images, we synthesize the image of the scene from a novel viewpoint. We minimize the photometric error between the warped image and the image at the given viewpoint.

Given a target view I t and S source views I s , we minimize the photometric error between the target view and the source view warped into the target's frame, denoted by ˆ I s . Mathematically, this can be described by Eq. 3

<!-- formula-not-decoded -->

where N is the total number of pixels.

## 3.3 Depth Consistency

Our depth network takes 2 views as input and outputs the depth w.r.t. the first image. In an iteration, we predict depths with each of the S source views as the second input and use the respective outputs to warp a given source frame into the target frame. Since all the predicted depths are for the target image itself, they would need to be consistent with one another. Therefore we minimize the depth error between the predicted depth images obtained using all source images.

<!-- formula-not-decoded -->

where N is the total number of pixels.

## 3.4 Spatial Smoothing

In order to tackle the issues of learning wrong depth values for texture-less regions, we try to ensure that the depth prediction is derived from spatially similar areas. One more thing to note is that depth discontinuities usually occur at object boundaries. We minimize L 1 norm of the 2 nd order spatial gradients of the inverse depth of a pixel ∂ 2 d ( p ) , weighted by the image laplacian at that pixel ∂ 2 I ( p ) . This is to account for sudden changes in depth due to crossing of object boundaries and ensure a smooth change in the

depth values. This is similar to what is done in [16, 41].

<!-- formula-not-decoded -->

where N is the total number of pixels.

## 3.5 Epipolar Constraints

The problem with simply minimizing such photometric errors is that it doesn't take ambiguous pixels into consideration, such as those belonging to non-rigid objects, those which are occluded etc. Thus, we need to weight pixels appropriately based on whether they're properly projected or not. One way of ensuring correct projection is by checking if the corresponding pixel ˆ p satisfies epipolar constraints or not, according to Eq. 1.

We impose epipolar constraints using the Essential Matrix obtained from Nistér's Five Point Algorithm [34] using matches between features extracted using SiftGPU [43]. This helps ensure that the warped pixels to lie on their corresponding epipolar line. This epipolar loss ˆ ˜ p T E ˜ p is used to weight the above losses, where E is the Essential Matrix obtained using the Five Point Algorithm. After weighting, the new photometric loss now becomes

<!-- formula-not-decoded -->

The reason behind this is that for a non-rigid object, even if the pixel is properly projected, the photometric error would be high. In order to ensure that such pixels are given a low weight, we weight them with their epipolar distance, which would be low if a pixel is properly projected. If the epipolar loss is high, it means that the projection is wrong, giving a high weight to the photometric loss, thereby increasing its overall penalty. This also helps in mitigating the problem of a pixel getting projected to a region of similar intensity by constraining it to lie along the epipolar line.

## 3.6 Structural Similarity

Another well known and robust metric for measuring perceptual differences between two images is the Structural Similarity Index (SSIM) [42]. It is widely applied in tasks that require comparing 2 images of the same scene, like comparing transmission quality. The photometric loss assumes brightness constancy which need not hold in all cases. Instead, SSIM considers three main factors, namely lunimance, constrast and structure, which provide a more robust measure for image similarity. Since SSIM needs to be maximized (with 1 as the maximum value), we minimize the below loss

<!-- formula-not-decoded -->

## 3.7 Final Loss

Our final loss function is a weighted combination of the above loss functions summed over multiple image scales.

<!-- formula-not-decoded -->

where l iterates over the different scale values and λ smooth λ ssim and λ depth are the the relative weights for the smoothness loss, SSIM loss and the depth consistency loss respectively.

Note that we don't minimize the epipolar loss but use it for weighting the other losses. This way the network tries to implicitly minimize it as it would lead to a reduction in the overall loss.

## 4 IMPLEMENTATION DETAILS

## 4.1 Neural Network Design

We use networks similar to that of SfMLearner, except we remove their "explainability mask" from the pose network. The network architectures that we use are shown in the appendix in Fig. A1.

## 4.2 Depth Network

The design of the Depth CNN is similar to the one in [47], which is inspired from DispNet[31]. It consists of an encoder-decoder network with skip connections from previous layers. The input is a pair of RGB images concatenated along the colour channel ( H × W × 6) and the output is the inverse depth of the first image. The idea behind this is that rather than just learning semantic artifacts in the scene from a single view, the network can learn inter-pixel relationships and correspondences, similar to how optical flow networks are modelled. Even in Visual SLAM/Odometry methods, multiple images are used to predict depth, instead of a single image.

Along with this, we also normalize the predicted inverse depth to have unit mean, to remove any scale ambiguity in the predicted depths. This is inspired from what is applied to the inverse depth of keyframes in LSD-SLAM[8]. As mentioned in [15], and as we observed in our experiments as well, performing a simple multiscale estimation causes "holes" to develop in textureless regions. They argue that minimizing photometric error would allow the network to predict incorrect depth at a lower scale, which would still end up with a low photometric error at that scale due to the textureless property of the region but lead to a larger photometric error at a higher resolution. Thus they propose to overcome this by upsampling the depth images to the input's resolution and then calculating the errors.

## 4.3 Pose Network

For the pose network, the target view and the source views are concatenated along the colour channel giving rise to an input layer of size H × W × 3 N where N is the number of input views. The network predicts 6 DoF poses for each of the N -1 source views relative to the target image. We modify the pose network proposed by [47] by removing their "explainability mask" thereby having to learn lesser parameters yet giving better performance.

## 4.4 Training

We use Tensorflow [1] for implementing the system. We use batch normalization[21] for the non-output layers and make use of the Adam Optimizer [26] with β 1 = 0 . 9, β 2 = 0 . 999 and a learning rate of 0.0002 and a mini-batch of size 4 for training our networks. We set the weights as λ smooth = 0 . 2, λ ssim = 0 . 7 and λ depth = 0 . 5. The learning typically converges after 26 epochs. We use raw images from the KITTI dataset[13], with the split given by [6], having

Image

Ground truth

SfMLearner

Epipolar Geometry based Learning of Multi-view Depth and Ego-Motion from Monocular Sequences

Proposed Method

ICVGIP 2018, December 18-22 2018, Hyderabad, India

Figure 2: Results of depth estimation compared with SfMLearner. The ground truth is interpolated from sparse measurements for visualization purposes. Some of their main failure cases of SfMLearner are highlighted in the last 3 figures, such as large open spaces, texture-less regions, and when objects are present right in front of the camera. As it can be seen in the last 3 figures, our method performs better, providing more meaningful depth estimates even in such scenarios. (Pictures best viewed in color.)

<!-- image -->

about 40K images totally. We exclude static scenes and test image sequences from our training set leaving us with 33K images. We use 3 views as the input to the pose network with the middle image as our target image and the previous and next images as the source images. We use 2 views as input to the depth network with the middle image as the target view and predict it's depth with both the source images as the second input, one by one.

## 5 RESULTS

## 5.1 Depth Estimation Results

We evaluate our performance on the 697 images provided by [6]. We show our results in Table 1. Our method's performance exceeds that of SfMLearner[47], Yang et. al[45], Kuznietsov et. al[28] (only monocular) which are purely monocular. We also perform better than methods which use depth supervision [6, 29] and [12] who

Table 1: Single View Depth results using the split of [6]. [12] cap their depth at 50m which we show in the bottom part of the table. Further details about the error and accuracy metrics can be found in [6]. The dashed line separates methods that use some form of supervision from purely monocular methods. Metrics are explained in the appendix in Sec. 5.2. Baseline numbers taken from [28, 45, 47].

| Method                                 | Supervision   | Error Metric (lower is better)   | Error Metric (lower is better)   | Error Metric (lower is better)   | Error Metric (lower is better)   | Accuracy Metric (higher is better)   | Accuracy Metric (higher is better)   | Accuracy Metric (higher is better)   |
|----------------------------------------|---------------|----------------------------------|----------------------------------|----------------------------------|----------------------------------|--------------------------------------|--------------------------------------|--------------------------------------|
| Method                                 | Supervision   | Abs. Rel.                        | Sq. Rel.                         | RMSE                             | RMSE log                         | δ < 1 . 25                           | δ < 1 . 25 2                         | δ < 1 . 25 3                         |
| Train set mean                         | -             | 0.403                            | 5.53                             | 8.709                            | 0.403                            | 0.593                                | 0.776                                | 0.878                                |
| Eigen et. al [6] Coarse                | Depth         | 0.214                            | 1.605                            | 6.563                            | 0.292                            | 0.673                                | 0.884                                | 0.957                                |
| Eigen et. al [6] Fine                  | Depth         | 0.203                            | 1.548                            | 6.307                            | 0.282                            | 0.702                                | 0.89                                 | 0.958                                |
| Liu et. al [29]                        | Depth         | 0.202                            | 1.614                            | 6.523                            | 0.275                            | 0.678                                | 0.895                                | 0.965                                |
| Godard et. al[16]                      | Stereo        | 0.148                            | 1.344                            | 5.927                            | 0.247                            | 0.803                                | 0.922                                | 0.964                                |
| Kuznietsov et. al[28] (Only Monocular) | Mono          | 0.308                            | 9.367                            | 8.700                            | 0.367                            | 0.752                                | 0.904                                | 0.952                                |
| Yang et. al[45]                        | Mono          | 0.182                            | 1.481                            | 6.501                            | 0.267                            | 0.725                                | 0.906                                | 0.963                                |
| SfMLearner [47] (w/o explainability)   | Mono          | 0.221                            | 2.226                            | 7.527                            | 0.294                            | 0.676                                | 0.885                                | 0.954                                |
| SfMLearner [47]                        | Mono          | 0.208                            | 1.768                            | 6.856                            | 0.283                            | 0.678                                | 0.885                                | 0.957                                |
| SfMLearner [47] (updated from github)  | Mono          | 0.183                            | 1.595                            | 6.709                            | 0.270                            | 0.734                                | 0.902                                | 0.959                                |
| Ours                                   | Mono          | 0.175                            | 1.675                            | 6.378                            | 0.255                            | 0.760                                | 0.916                                | 0.966                                |
| Garg et. al [12]                       | Stereo        | 0.169                            | 1.08                             | 5.104                            | 0.273                            | 0.74                                 | 0.904                                | 0.962                                |
| SfMLearner [47] (w/o explainability)   | Mono          | 0.208                            | 1.551                            | 5.452                            | 0.273                            | 0.695                                | 0.900                                | 0.964                                |
| SfMLearner [47]                        | Mono          | 0.201                            | 1.391                            | 5.181                            | 0.264                            | 0.696                                | 0.900                                | 0.966                                |
| Ours                                   | Mono          | 0.166                            | 1.213                            | 4.812                            | 0.239                            | 0.777                                | 0.928                                | 0.972                                |

.

use calibrated stereo supervision. We fall short of [16], who use calibrated stereo supervision along with left-right consistency, which makes their approach more robust.

The images after the red line in Fig. 2 are cases where our method performs better in places where SfMLearner fails, such as textureless scenes and open regions. This shows the effectiveness of having 2-view depth prediction and using epipolar geometry to handle occlusions and non-rigidity. We provide sharper outputs as compared to SfMLearner, which can be seen in the which is the result of using an edge-aware smoothness that helps capture the shape of objects in a better manner. We scale our depth predictions such that it matches the median of the ground truth. Further explanation about the metrics can be found is [6], which are given below in Sec. 5.2

## 5.2 Depth Evaluation Metrics

Given the predicted depth ˆ y i and the corresponding ground truth depth y ∗ i for the i th image, we use the following error metrics and accuracy metrics.

## 5.2.1 Error Metrics.

- Absolute Relative Difference (Abs. Rel.): 1 ˝ N = 1 | ˆ y i -y ∗ |/ y ∗
- Root Mean Squared Error (RMSE): q 1 N ˝ N i = 1 | | ˆ y i -y ∗ i | | 2
- N i i i · Squared Relative Difference (Sq. Rel.): 1 N ˝ N i = 1 | | ˆ y i -y ∗ i | | 2 / y ∗ i
- Logarithmic RMSE (RMSE log): q 1 N ˝ N i = 1 | | log ˆ y i -log y ∗ i | | 2

5.2.2 Accuracy Metrics. We calculate the accuracy metric as the percentage of images for which the value of δ , defined as δ = max ( ˆ y i y ∗ i , y ∗ i ˆ y i ) , is lesser than a threshold th . In our case, we choose three values for the threshold th which are 1 . 25, 1 . 25 2 and 1 . 25 3 .

## 5.3 Pose Estimation Results

For our pose estimation experiments, we use the KITTI Visual Odometry Benchmark dataset [14]. Only 11 sequences (00-10) have the associated ground truth data, on which we show our results. We use a sequence of 3 views as the input to the pose network with the middle view as the target view. Each image is of size 1271 × 376 which we scale down to a size of 416 × 128 for both pose estimation and depth estimation experiments.

Weshow the Average Trajectory Error (ATE) and Average Translational Direction Error (ATDE) averaged over 3 frame intervals. Before comparison, the scale is first corrected to best align it with the ground truth after which the ATE is computed. Since the ATDE is only comparing the angle between the directions of translations, we do not correct the scale of the poses.

Table 2 shows the results of our pose estimation. We perform better than SfMLearner 1 on an average in terms of the ATE showing that adding meaningful geometric constraints helps get better estimates as compared to minimzing just the reprojection error. We perform better on all runs compared to the relative poses obtained from the Five Point Algorithm in terms of the ATDE.

This rises from the fact that, we have additional constraints of depth and image warping that help give a better estimation of the direction of motion, whereas the Five point Algorithm uses only sparse point correspondences between the images. Moreover, the Five point Algorithm itself is slightly erroneous due to inaccuracies arising in the feature matching or in the RANSAC based estimation of the essential matrix. Despite being given slightly erroneous estimates of the essential matrix, incorporating image reconstruction as the main goal helps in overcoming erroneous predictions.

1 using the model provided at github.com/tinghuiz/SfMLearner

Table 2: Average Trajectory Error (ATE) compared with SfMLearner and Average Translational Direction Error (ATDE) compared with the Five Point Algorithm averaged over 3 frame snippets on the KITTI Visual Odometry Dataset [14]. The ATE is shown in meters and the ATDE, in radians. All values are reported as mean ± std. dev.

|                                  | Average Trajectory Error                                                                                                                                                        | Average Trajectory Error                                                                                                                                                        | Average Translational Direction Error                                                                                                                                           | Average Translational Direction Error                                                                                                                                           |
|----------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Seq                              | SfMLearner[47]                                                                                                                                                                  | Ours                                                                                                                                                                            | Five Point Algorithm[34] using SiftGPU[43]                                                                                                                                      | Ours                                                                                                                                                                            |
| 00 01 02 03 04 05 06 07 08 09 10 | 0.5099 ± 0.2471 1.2290 ± 0.2518 0.6330 ± 0.2328 0.3767 ± 0.1527 0.4869 ± 0.0537 0.5013 ± 0.2564 0.5027 ± 0.2605 0.4337 ± 0.3254 0.4824 ± 0.2396 0.6652 ± 0.2863 0.4672 ± 0.2398 | 0.4967 ± 0.1787 1.1458 ± 0.2175 0.6512 ± 0.1806 0.3583 ± 0.1254 0.6404 ± 0.0607 0.4930 ± 0.1974 0.5384 ± 0.1627 0.4032 ± 0.2380 0.4708 ± 0.1827 0.6280 ± 0.2028 0.4185 ± 0.1791 | 0.0084 ± 0.0821 0.0061 ± 0.0807 0.0035 ± 0.0509 0.0142 ± 0.1611 0.0182 ± 0.2131 0.0130 ± 0.0945 0.0130 ± 0.1591 0.0508 ± 0.2453 0.0091 ± 0.0646 0.0204 ± 0.1722 0.0200 ± 0.1241 | 0.0040 ± 0.0155 0.0033 ± 0.0077 0.0021 ± 0.0026 0.0027 ± 0.0042 0.0002 ± 0.0007 0.0044 ± 0.0044 0.0080 ± 0.0688 0.0114 ± 0.0430 0.0037 ± 0.0058 0.0073 ± 0.0211 0.0040 ± 0.0105 |

Table 3: Ablative study on the effect of different losses in depth estimation. We show our results using the split of [6] while removing the proposed losses. We compare our method with SfMLearner (w/o explainability) which is essentially similar to stripping our method of the proposed losses and using a different smoothness loss.

| Method                          | Error Metric (lower is better)   | Error Metric (lower is better)   | Error Metric (lower is better)   | Error Metric (lower is better)   | Accuracy Metric (higher is better)   | Accuracy Metric (higher is better)   | Accuracy Metric (higher is better)   |
|---------------------------------|----------------------------------|----------------------------------|----------------------------------|----------------------------------|--------------------------------------|--------------------------------------|--------------------------------------|
| Method                          | Abs. Rel.                        | Sq. Rel.                         | RMSE                             | RMSE log                         | δ < 1 . 25                           | δ < 1 . 25 2                         | δ < 1 . 25 3                         |
| SfMLearner (w/o explainability) | 0.221                            | 2.226                            | 7.527                            | 0.294                            | 0.676                                | 0.885                                | 0.954                                |
| Ours (no-epi)                   | 0.199                            | 1.548                            | 6.314                            | 0.274                            | 0.697                                | 0.901                                | 0.964                                |
| Ours (single-view)              | 0.181                            | 1.520                            | 6.080                            | 0.260                            | 0.747                                | 0.914                                | 0.965                                |
| Ours (1 st order)               | 0.190                            | 1.440                            | 6.144                            | 0.269                            | 0.714                                | 0.906                                | 0.965                                |
| Ours (final)                    | 0.175                            | 1.675                            | 6.378                            | 0.255                            | 0.761                                | 0.916                                | 0.966                                |

## 6 ABLATION STUDY

In order to see what is the contribution of our proposed loss to the learning process, we perform an ablation study on the depth estimation by considering variants of our proposed approach and training the networks using the Eigen[6] split. We show the results of studying the effects of epipolar constraints, 2-view depth prediction and the second order edge-based smoothness.

In order to see the effectiveness of our method, we first replace the second order edge-aware smoothness with a first order edgebased smoothness. We call this as "Ours (1 st order)". Then we remove the epipolar loss which is similar to having an SfMLearner pipeline with 2 views as input to the depth network. We denote this as "Ours (no-epi)". Then in order to see the contribution of our proposed idea of using 2 views as input to the depth network, we train a variant with a single view depth having epipolar constraints and a first order edge-based smoothness. This is denoted as "Ours (single-view)".

Any further removal would just result in having SfMLearner without the explainability mask, which we consider as our baseline. This is essentially equivalent to removing our proposed improvements and replacing the edge-aware depth with a simpler second order smoothness loss. Finally, we further investigate the possibility of using 3 views as the input to the depth network as well.

The results of the study are shown in Table 3. All our variants perform significantly better than the standard SfMLearner without the explainability mask showing that our method has a positive effect on the learning. Just adding 2-view depth prediction without epipolar constraints (Ours no-epi) gives a significant improvement showing that using multiple views provide better depth estimates, compared to a single view. Incorporating just the epipolar loss (Ours single-view) improves the output drastically, showing that geometrically meaningful constraints provide better depth outputs.

.

When we combine the two, it doesn't lead to a drastic improvement over the individual methods, however providing a second order smoothness improves the results as compared to a first order smoothness. A first order smoothness implies having a constant depth change, which isn't necessarily true. For parts of the image that are closer to the camera, the depth varies slowly, whereas for those which are further away, the depth variations are larger. Therefore, having a second order smoothness captures this change in depth variation rather than just the change in depth.

Figure 3: Results of 3-view depth estimation. As it can be seen, though the coarse structure of the scene is detected, objects are still not as finely reconstructed. The third image is an example where the car is taking a turn, which gives haphazard depth outputs.

<!-- image -->

Figure 4: Results of 3-view depth estimation with using larger convolutional filter sizes. It gives a smoother depth output as before, however, it still isn't able to fully capture the scene depth. The third image is an example where the car is taking a turn, which still gives haphazard depth outputs.

<!-- image -->

## 6.1 Three-view Depth Prediction

One more variant which we tried out was using 3 views for the depth prediction as well. Though it was giving visually understandable results, our 2-view variant was giving better depth estimates. One observation is that during turning, the depth outputs would deteriorate at a much larger scale. A possible explanation for this could be that compared to the motion between 2 views, while using 3 views, there is a larger amount of motion, and thereby lesser amount of overlap between images. Though the motion is not too large, it is large enough to escape the field of view of the convolutional filter in an input, since the views are stacked together and given to the network. Some sample outputs of using a 3 view depth prediction are shown in Fig. 3.

In order to test our hypothesis regarding the filter sizes, we tried increasing the filter sizes and performing the depth prediction. More specifically, we increase each of the filter sizes by a value of 4. By doing so, we effectively increase the perceptive field of view of a filter thus allowing it to accumulate information from a larger area of pixels. This way, it would be able to properly "see" a pixel across multiple views, which would otherwise fall outside the field of view of a filter. We observed that this leads to smoother depth images as compared to using smaller filter sizes. However, it still didn't perform as well as the 2-view variant, and it ended up developing a few unwanted holes as well. These results are shown in Fig. 4.

## 7 CONCLUSION AND FUTURE WORK

We build upon a previous unsupervised method for learning deep monocular visual odometry and depth by leveraging the fact that depth estimation could be made more robust by using multiple views rather than a single view. Along with this, we incorporate epipolar constraints to help make the learning more geometrically meaningful while using lesser number of trainable parameters. Our method is able to predict depth with higher accuracy along with giving sharper depth estimates and better pose estimates. Although increasing the number of inputs for depth prediction gave a good output in 2 views, it's 3-view counterpart wasn't able to perform as well. This would be an interesting problem to look into for improving it, either by architectural changes in the depth network or by incorporating a post-processing optimization on top of the networks.

The current method however only performs pixel level inferences. A higher scene level understanding can be obtained by integrating semantics of the scene to get better correlation between objects in the scene and the depth and ego-motion estimates. This is similar to using semantic motion segmentation [17, 18]. Architectural changes could also be leveraged to get a stronger coupling between depth and pose by having a single network predicting both pose and depth in order to allow the network to be able to learn representations that capture the complex relation between both camera motion and scene depth.

## REFERENCES

- [1] Martín Abadi, Paul Barham, Jianmin Chen, Zhifeng Chen, Andy Davis, Jeffrey Dean, Matthieu Devin, Sanjay Ghemawat, Geoffrey Irving, Michael Isard, et al. 2016. TensorFlow: A System for Large-Scale Machine Learning.. In Operating Systems Design and Implementation (OSDI) .
- [2] Hatem Alismail, Brett Browning, and Simon Lucey. 2016. Enhancing direct camera tracking with dense feature descriptors. In Asian Conference on Computer Vision (ACCV) .
- [3] Brojeshwar Bhowmick, Suvam Patra, Avishek Chatterjee, Venu Madhav Govindu, and Subhashis Banerjee. 2014. Divide and conquer: Efficient large-scale structure from motion using graph partitioning. In Asian Conference on Computer Vision (ACCV) .
- [4] Arunkumar Byravan and Dieter Fox. 2017. Se3-nets: Learning rigid body motion using deep neural networks. In IEEE International Conference on Robotics and Automation (ICRA) .
- [5] Vishakh Duggal, Kumar Bipin, Utsav Shah, and K Madhava Krishna. 2016. Hierarchical structured learning for indoor autonomous navigation of Quadcopter. In Indian Conference on Computer Vision, Graphics and Image Processing (ICVGIP) .
- [6] David Eigen, Christian Puhrsch, and Rob Fergus. 2014. Depth map prediction from a single image using a multi-scale deep network. In Advances in Neural Information Processing Systems (NIPS) .
- [7] Jakob Engel, Vladlen Koltun, and Daniel Cremers. 2018. Direct Sparse Odometry. IEEE Transactions on Pattern Analysis and Machine Intelligence (2018).
- [8] Jakob Engel, Thomas Schöps, and Daniel Cremers. 2014. LSD-SLAM: Large-scale Direct Monocular SLAM. In European Conference on Computer Vision (ECCV) .
- [9] Martin A Fischler and Robert C Bolles. 1981. Random sample consensus: a paradigm for model fitting with applications to image analysis and automated cartography. Commun. ACM (1981).
- [10] Christian Forster, Matia Pizzoli, and Davide Scaramuzza. 2014. SVO: Fast SemiDirect Monocular Visual Odometry. In IEEE International Conference on Robotics and Automation (ICRA) .
- [11] Yasutaka Furukawa, Brian Curless, Steven M Seitz, and Richard Szeliski. 2010. Towards internet-scale multi-view stereo. In IEEE Conference on Computer Vision and Pattern Recognition (CVPR) .
- [12] Ravi Garg, Vijay Kumar BG, Gustavo Carneiro, and Ian Reid. 2016. Unsupervised cnn for single view depth estimation: Geometry to the rescue. In European Conference on Computer Vision (ECCV) .
- [13] Andreas Geiger, Philip Lenz, Christoph Stiller, and Raquel Urtasun. 2013. Vision meets Robotics: The KITTI Dataset. The International Journal of Robotics Research (IJRR) (2013).
- [14] Andreas Geiger, Philip Lenz, and Raquel Urtasun. 2012. Are we ready for Autonomous Driving? The KITTI Vision Benchmark Suite. In IEEE Conference on Computer Vision and Pattern Recognition (CVPR) .
- [15] Clément Godard, Oisin Mac Aodha, and Gabriel Brostow. 2018. Digging Into Self-Supervised Monocular Depth Estimation. arXiv preprint arXiv:1806.01260 (2018).
- [16] Clément Godard, Oisin Mac Aodha, and Gabriel J Brostow. 2017. Unsupervised monocular depth estimation with left-right consistency. In IEEE Conference on Computer Vision and Pattern Recognition (CVPR) .
- [17] Nazrul Haque, N Dinesh Reddy, and K Madhava Krishna. 2017. Joint Semantic and Motion Segmentation for dynamic scenes using Deep Convolutional Networks. In International Joint Conference on Computer Vision, Imaging and Computer Graphics Theory and Applications (VISAPP) .
- [18] Nazrul Haque, N Dinesh Reddy, and Madhava Krishna. 2017. Temporal Semantic Motion Segmentation Using Spatio Temporal Optimization. In International Workshop on Energy Minimization Methods in Computer Vision and Pattern Recognition (EMMCVPR) .
- [19] Richard Hartley and Andrew Zisserman. 2003. Multiple view geometry in computer vision . Cambridge university press.
- [20] Derek Hoiem, Alexei A Efros, and Martial Hebert. 2005. Automatic photo pop-up. In ACM Transactions on Graphics (TOG) .
- [21] Sergey Ioffe and Christian Szegedy. 2015. Batch normalization: Accelerating deep network training by reducing internal covariate shift. International Conference on Machine Learning (ICML) (2015).
- [22] Max Jaderberg, Karen Simonyan, Andrew Zisserman, et al. 2015. Spatial transformer networks. In Advances in Neural Information Processing Systems (NIPS) .
- [23] J Yu Jason, Adam W Harley, and Konstantinos G Derpanis. 2016. Back to basics: Unsupervised learning of optical flow via brightness constancy and motion smoothness. In European Conference on Computer Vision (ECCV) .
- [24] Justin Johnson, Andrej Karpathy, and Li Fei-Fei. 2016. Densecap: Fully convolutional localization networks for dense captioning. In IEEE Conference on Computer Vision and Pattern Recognition (CVPR) .
- [25] Juan Jose Tarrio and Sol Pedre. 2015. Realtime edge-based visual odometry for a monocular camera. In IEEE International Conference on Computer Vision (ICCV) .
- [26] Diederik P Kingma and Jimmy Ba. 2015. Adam: A method for stochastic optimization. In International Conference on Learning Representations (ICLR) .

ICVGIP 2018, December 18-22 2018, Hyderabad, India

- [27] Georg Klein and David Murray. 2007. Parallel Tracking And Mapping for Small AR Workspaces. In International Symposium on Mixed and Augmented Reality (ISMAR) .
- [28] Yevhen Kuznietsov, Jörg Stückler, and Bastian Leibe. 2017. Semi-supervised deep learning for monocular depth map prediction. In IEEE Conference on Computer Vision and Pattern Recognition (CVPR) .
- [29] Fayao Liu, Chunhua Shen, Guosheng Lin, and Ian Reid. 2016. Learning depth from single monocular images using deep convolutional neural fields. IEEE Transactions on Pattern Analysis and Machine Intelligence (2016).
- [30] Soumyadip Maity, Arindam Saha, Brojeshwar Bhowmick, Chanoh Park, Soohwan Kim, Peyman Moghadam, Clinton Fookes, Sridha Sridharan, Ivan Eichhardt, Levente Hajder, et al. 2017. Edge SLAM: Edge Points Based Monocular Visual SLAM. In IEEE International Conference on Computer Vision (ICCV) Workshops (IEEE International Conference on Computer Vision (ICCV)W) .
- [31] Nikolaus Mayer, Eddy Ilg, Philip Hausser, Philipp Fischer, Daniel Cremers, Alexey Dosovitskiy, and Thomas Brox. 2016. A large dataset to train convolutional networks for disparity, optical flow, and scene flow estimation. In IEEE Conference on Computer Vision and Pattern Recognition (CVPR) .
- [32] Raul Mur-Artal, Jose Maria Martinez Montiel, and Juan D Tardos. 2015. ORBSLAM: a versatile and accurate monocular SLAM system. IEEE Transactions on Robotics (TRO) (2015).
- [33] Richard A Newcombe, Steven J Lovegrove, and Andrew J Davison. 2011. DTAM: Dense tracking and mapping in real-time. In IEEE International Conference on Computer Vision (ICCV) .
- [34] David Nistér. 2004. An efficient solution to the five-point relative pose problem. IEEE Transactions on Pattern Analysis and Machine Intelligence (2004).
- [35] Viorica Patraucean, Ankur Handa, and Roberto Cipolla. 2015. Spatio-temporal video autoencoder with differentiable memory. arXiv preprint arXiv:1511.06309 (2015).
- [36] Aseem Saxena, Harit Pandya, Gourav Kumar, Ayush Gaud, and K Madhava Krishna. 2017. Exploring convolutional networks for end-to-end visual servoing. In IEEE International Conference on Robotics and Automation (ICRA) .
- [37] Ashutosh Saxena, Min Sun, and Andrew Y Ng. 2007. Learning 3-d scene structure from a single still image. In IEEE International Conference on Computer Vision (ICCV) .
- [38] Peter Sturm and Bill Triggs. 1996. A factorization based algorithm for multiimage projective structure and motion. In European Conference on Computer Vision (ECCV) .
- [39] Benjamin Ummenhofer, Huizhong Zhou, Jonas Uhrig, Nikolaus Mayer, Eddy Ilg, Alexey Dosovitskiy, and Thomas Brox. 2017. Demon: Depth and motion network for learning monocular stereo. In IEEE Conference on Computer Vision and Pattern Recognition (CVPR) .
- [40] Sudheendra Vijayanarasimhan, Susanna Ricco, Cordelia Schmid, Rahul Sukthankar, and Katerina Fragkiadaki. 2017. Sfm-net: Learning of structure and motion from video. arXiv:1704.07804 (2017).
- [41] Chaoyang Wang, José Miguel Buenaposada, Rui Zhu, and Simon Lucey. 2018. Learning Depth from Monocular Videos using Direct Methods. In IEEE Conference on Computer Vision and Pattern Recognition (CVPR) .
- [42] Zhou Wang, Alan C Bovik, Hamid R Sheikh, and Eero P Simoncelli. 2004. Image quality assessment: from error visibility to structural similarity. IEEE Transactions on Image Processing (2004).
- [43] Changchang Wu. 2007. SiftGPU: A GPU implementation of sift. (2007). http: //cs.unc.edu/~ccwu/siftgpu
- [44] Changchang Wu. 2013. Towards linear-time incremental structure from motion. In IEEE International Conference on 3D Vision (3DV) .
- [45] Zhenheng Yang, Peng Wang, Wei Xu, Liang Zhao, and Ramakant Nevatia. 2018. Unsupervised Learning of Geometry From Videos With Edge-Aware DepthNormal Consistency. In AAAI .
- [46] Zhichao Yin and Jianping Shi. 2018. GeoNet: Unsupervised Learning of Dense Depth, Optical Flow and Camera Pose. In IEEE Conference on Computer Vision and Pattern Recognition (CVPR) .
- [47] Tinghui Zhou, Matthew Brown, Noah Snavely, and David G Lowe. 2017. Unsupervised learning of depth and ego-motion from video. In IEEE Conference on Computer Vision and Pattern Recognition (CVPR) .

## Appendix for "Epipolar Geometry based Learning of Multi-view Depth and Ego-Motion from Monocular Sequences"

<!-- image -->

(a) The Depth network has a design of a convolutional-deconvolutional encoder-decoder network with skip connections from previous layers. The first 4 layers have kernel sizes 7,7,5,5 respectively. The rest of the layers have a kernel of size 3. The number of output channels for the first layer is 32 and increases by a factor of 2 after each layer until it reaches 512 following which it stays the same. The decoder uses a sequence of the following layers. First a deconvolution is performed followed by a convolution of the concatenation of the current layer with the corresponding layer in the encoder. This is done for the first 4 deconv-conv sequences after which the output inverse-depth prediction gets upsampled and concatenated. The first two deconv-conv sequences have a 512 output channels which gets reduced by a factor of 2 for each subsequent sequence. The output layers are single channel convolutional layers with a kernel size 3 and stride 1. The depth output is predicted at 4 scales. The strides alternate between 2 and 1 for non-output layers in the whole network. We use ReLU activations except for the output layers where we use sigmoid functions of the form Sσ ( x ) + ∆ where S is a scaling factor which we keep as 10 to keep the output in a reasonable range, and ∆ is an offset which we keep as 0.01 to ensure positive non-zero outputs. We apply an inverse-depth normalization to scale the predicted inverse depth to have unit mean.

<!-- image -->

(b) The Pose network consists of 7 convolutional layers with ReLU activation followed be a single stride output layer with no activation. All layers have a kernel size of 3 except the first 2 layers having kernel sizes of 7 and 5 respectively. The number of output channels of the first layer is 16 and increases by a factor of 2. Global average pooling is applied to the network output.

Figure A1: Neural Network architectures for (a) the depth network and (b) the pose network. The change in width/height between the layers indicates a increase/decrease by a factor of 2.