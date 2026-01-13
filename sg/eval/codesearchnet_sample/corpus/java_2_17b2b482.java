@Override
  public C instantiateClass(Parameterization config) {
    return (instance != null) ? instance : (instance = super.instantiateClass(config));
    // NOTE: instance may remain null here, when instantiateClass failed.
  }