def percent( inputValues )
      raise "'inputValues' must not be nil" if inputValues.nil?
      raise "'inputValues' must be an Array" if not inputValues.is_a?(Array)
      raise "'inputValues' must have at least two elements" if inputValues.length < 2
      total = inputValues[0].to_f
      total = 1.0 if total == 0.00
      value = inputValues[1].to_f
      ((value/total)*100)
   end