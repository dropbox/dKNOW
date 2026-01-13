public double[][] diff_y(double maty[][]) {
        mat3 = new double[width][height];
        double mat1, mat2;
        for (int i = 0; i < width; i++) {
            for (int j = 0; j < height; j++) {
                if (i == 0) {
                    mat1 = maty[i][j];
                    mat2 = maty[i + 1][j];
                } else if (i == width - 1) {
                    mat1 = maty[i - 1][j];
                    mat2 = maty[i][j];
                } else {
                    mat1 = maty[i - 1][j];
                    mat2 = maty[i + 1][j];
                }
                mat3[i][j] = (mat2 - mat1) / 2;
            }
        }
        // maty= subMatrix(mat2,mat1,width,height);

        return mat3;
    }