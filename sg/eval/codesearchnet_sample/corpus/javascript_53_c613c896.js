function (configFile, flagsNames, extraOptions, usedConfigs) {
    var configDir = path.dirname(configFile),
        rawConfig = readConfig(configFile);

    configFile = fs.realpathSync(configFile);
    return assembleLmdConfigAsObject(rawConfig, configFile, configDir, flagsNames, extraOptions, usedConfigs);
}