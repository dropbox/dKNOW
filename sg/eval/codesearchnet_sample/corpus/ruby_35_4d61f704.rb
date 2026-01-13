def check_for_too_many_processors(config, hash)
      concurrency = config.concurrency

      errors = []
      Overcommit::Utils.supported_hook_type_classes.each do |hook_type|
        hash.fetch(hook_type) { {} }.each do |hook_name, hook_config|
          processors = hook_config.fetch('processors') { 1 }
          if processors > concurrency
            errors << "#{hook_type}::#{hook_name} `processors` value " \
                      "(#{processors}) is larger than the global `concurrency` " \
                      "option (#{concurrency})"
          end
        end
      end

      if errors.any?
        if @log
          @log.error errors.join("\n")
          @log.newline
        end
        raise Overcommit::Exceptions::ConfigurationError,
              'One or more hooks had invalid `processor` value configured'
      end
    end