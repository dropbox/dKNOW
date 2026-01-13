public static JavaConstant ofPrimitiveType(TypeDescription typeDescription) {
            if (!typeDescription.isPrimitive()) {
                throw new IllegalArgumentException("Not a primitive type: " + typeDescription);
            }
            return new Dynamic(new ConstantDynamic(typeDescription.getDescriptor(),
                    TypeDescription.CLASS.getDescriptor(),
                    new Handle(Opcodes.H_INVOKESTATIC,
                            CONSTANT_BOOTSTRAPS,
                            "primitiveClass",
                            "(Ljava/lang/invoke/MethodHandles$Lookup;Ljava/lang/String;Ljava/lang/Class;)Ljava/lang/Class;",
                            false)), TypeDescription.CLASS);
        }