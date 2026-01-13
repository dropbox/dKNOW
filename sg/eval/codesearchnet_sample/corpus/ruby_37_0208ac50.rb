def merch_months_in(start_date, end_date) 
      merch_months_combos = merch_year_and_month_from_dates(start_date, end_date)
      merch_months_combos.map { | merch_month_combo | start_of_month(merch_month_combo[0], merch_month_combo[1]) }
    end