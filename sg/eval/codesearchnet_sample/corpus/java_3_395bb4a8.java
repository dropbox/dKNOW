private boolean varHasData(Variable v, StructureMembers sm) {
    if (sm.findMember(v.getShortName()) != null) return true;
    while (v instanceof VariableEnhanced) {
      VariableEnhanced ve = (VariableEnhanced) v;
      if (sm.findMember(ve.getOriginalName()) != null) return true;
      v = ve.getOriginalVariable();
    }
    return false;
  }