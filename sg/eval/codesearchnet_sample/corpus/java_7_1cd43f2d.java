public OutlierResult run(Relation<V> relation) {
    final int dim = RelationUtil.dimensionality(relation);
    final ArrayDBIDs ids = DBIDUtil.ensureArray(relation.getDBIDs());

    svm.svm_set_print_string_function(LOG_HELPER);

    svm_parameter param = new svm_parameter();
    param.svm_type = svm_parameter.ONE_CLASS;
    param.kernel_type = svm_parameter.LINEAR;
    param.degree = 3;
    switch(kernel){
    case LINEAR:
      param.kernel_type = svm_parameter.LINEAR;
      break;
    case QUADRATIC:
      param.kernel_type = svm_parameter.POLY;
      param.degree = 2;
      break;
    case CUBIC:
      param.kernel_type = svm_parameter.POLY;
      param.degree = 3;
      break;
    case RBF:
      param.kernel_type = svm_parameter.RBF;
      break;
    case SIGMOID:
      param.kernel_type = svm_parameter.SIGMOID;
      break;
    default:
      throw new AbortException("Invalid kernel parameter: " + kernel);
    }
    // TODO: expose additional parameters to the end user!
    param.nu = nu;
    param.coef0 = 0.;
    param.cache_size = 10000;
    param.C = 1;
    param.eps = 1e-4; // not used by one-class?
    param.p = 0.1; // not used by one-class?
    param.shrinking = 0;
    param.probability = 0;
    param.nr_weight = 0;
    param.weight_label = new int[0];
    param.weight = new double[0];
    param.gamma = 1. / dim;

    // Transform data:
    svm_problem prob = new svm_problem();
    prob.l = relation.size();
    prob.x = new svm_node[prob.l][];
    prob.y = new double[prob.l];
    {
      DBIDIter iter = ids.iter();
      for(int i = 0; i < prob.l && iter.valid(); iter.advance(), i++) {
        V vec = relation.get(iter);
        // TODO: support compact sparse vectors, too!
        svm_node[] x = new svm_node[dim];
        for(int d = 0; d < dim; d++) {
          x[d] = new svm_node();
          x[d].index = d + 1;
          x[d].value = vec.doubleValue(d);
        }
        prob.x[i] = x;
        prob.y[i] = +1;
      }
    }

    if(LOG.isVerbose()) {
      LOG.verbose("Training one-class SVM...");
    }
    String err = svm.svm_check_parameter(prob, param);
    if(err != null) {
      LOG.warning("svm_check_parameter: " + err);
    }
    svm_model model = svm.svm_train(prob, param);

    if(LOG.isVerbose()) {
      LOG.verbose("Predicting...");
    }
    WritableDoubleDataStore scores = DataStoreUtil.makeDoubleStorage(relation.getDBIDs(), DataStoreFactory.HINT_DB);
    DoubleMinMax mm = new DoubleMinMax();
    {
      DBIDIter iter = ids.iter();
      double[] buf = new double[svm.svm_get_nr_class(model)];
      for(int i = 0; i < prob.l && iter.valid(); iter.advance(), i++) {
        V vec = relation.get(iter);
        svm_node[] x = new svm_node[dim];
        for(int d = 0; d < dim; d++) {
          x[d] = new svm_node();
          x[d].index = d + 1;
          x[d].value = vec.doubleValue(d);
        }
        svm.svm_predict_values(model, x, buf);
        double score = -buf[0]; // / param.gamma; // Heuristic rescaling, sorry.
        // Unfortunately, libsvm one-class currently yields a binary decision.
        scores.putDouble(iter, score);
        mm.put(score);
      }
    }
    DoubleRelation scoreResult = new MaterializedDoubleRelation("One-Class SVM Decision", "svm-outlier", scores, ids);
    OutlierScoreMeta scoreMeta = new BasicOutlierScoreMeta(mm.getMin(), mm.getMax(), Double.NEGATIVE_INFINITY, Double.POSITIVE_INFINITY, 0.);
    return new OutlierResult(scoreMeta, scoreResult);
  }