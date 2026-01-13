public static <II extends ImageGray<II>>
	OrientationIntegral<II> sliding_ii( ConfigSlidingIntegral config , Class<II> integralType)
	{
		if( config == null )
			config = new ConfigSlidingIntegral();
		config.checkValidity();

		return (OrientationIntegral<II>)
				new ImplOrientationSlidingWindowIntegral(config.objectRadiusToScale,config.samplePeriod,
						config.windowSize,config.radius,config.weightSigma, config.sampleWidth,integralType);
	}