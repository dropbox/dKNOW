def perfom_query(iterator, &block)
      if example
        connection.by_example(example, options).each(&iterator)
      else
        connection.all(options).each(&iterator)
      end
    end