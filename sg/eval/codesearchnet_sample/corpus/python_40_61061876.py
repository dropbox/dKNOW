def mutex_opts(dict,ex_op):
    """Check for presence of mutually exclusive keys in a dict.

    Call: mutex_opts(dict,[[op1a,op1b],[op2a,op2b]...]"""
    for op1,op2 in ex_op:
        if op1 in dict and op2 in dict:
            raise ValueError,'\n*** ERROR in Arguments *** '\
                  'Options '+op1+' and '+op2+' are mutually exclusive.'