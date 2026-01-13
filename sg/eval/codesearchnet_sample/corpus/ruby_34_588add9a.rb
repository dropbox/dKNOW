def is_numeric(table_name, column_name)
			if @db.execute("SELECT #{column_name} from #{table_name} LIMIT 1").first.first.is_a? Numeric
				return true
			else
				return false
			end
		end