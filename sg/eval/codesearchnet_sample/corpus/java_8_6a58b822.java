public void registerAspectComponents() throws Exception {
		Debug.logVerbose("[JdonFramework] note: registe aspect components ", module);
		try {
			InterceptorsChain existedInterceptorsChain = (InterceptorsChain) containerWrapper.lookup(ComponentKeys.INTERCEPTOR_CHAIN);

			Iterator iter = aspectConfigComponents.iterator();
			Debug.logVerbose("[JdonFramework] 3 aspectConfigComponents size:" + aspectConfigComponents.size(), module);
			while (iter.hasNext()) {
				String name = (String) iter.next();
				AspectComponentsMetaDef componentMetaDef = (AspectComponentsMetaDef) aspectConfigComponents.getComponentMetaDef(name);
				// registe into container
				xmlcontainerRegistry.registerAspectComponentMetaDef(componentMetaDef);
				// got the interceptor instance;
				// add interceptor instance into InterceptorsChain object
				existedInterceptorsChain.addInterceptor(componentMetaDef.getPointcut(), name);
			}
		} catch (Exception ex) {
			Debug.logError("[JdonFramework] registerAspectComponents error:" + ex, module);
			throw new Exception(ex);
		}

	}