function fileClosed(file) {
        if (!file) {
            return;
        }
        var language = LanguageManager.getLanguageForPath(file._path),
            size = -1;

        function _sendData(fileSize) {
            var subType = "";

            if(fileSize/1024 <= 1) {

                if(fileSize < 0) {
                    subType = "";
                }
                if(fileSize <= 10) {
                    subType = "Size_0_10KB";
                } else if (fileSize <= 50) {
                    subType = "Size_10_50KB";
                } else if (fileSize <= 100) {
                    subType = "Size_50_100KB";
                } else if (fileSize <= 500) {
                    subType = "Size_100_500KB";
                } else {
                    subType = "Size_500KB_1MB";
                }

            } else {
                fileSize = fileSize/1024;
                if(fileSize <= 2) {
                    subType = "Size_1_2MB";
                } else if(fileSize <= 5) {
                    subType = "Size_2_5MB";
                } else {
                    subType = "Size_Above_5MB";
                }
            }

            sendAnalyticsData(commonStrings.USAGE + commonStrings.FILE_CLOSE + language._name + subType,
                                commonStrings.USAGE,
                                commonStrings.FILE_CLOSE,
                                language._name.toLowerCase(),
                                subType
                             );
        }

        file.stat(function(err, fileStat) {
            if(!err) {
                size = fileStat.size.valueOf()/1024;
            }
            _sendData(size);
        });
    }