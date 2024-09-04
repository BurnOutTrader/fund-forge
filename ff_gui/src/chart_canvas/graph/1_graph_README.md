# TimeSeriesData
* The data is drawn in batches using TimeSeriesData::draw_data()
* A matching statement is used for each data type which must be hardcoded to a draw fn
* Considering using a trait, but that will mean forcing the TimeSeriesData enum to only hold types which implement the trait, or use a dynamic  dispatch approach, or make a special 


1. make a simple indicator that goes onto the main canvas
2. make a simple indictor that goes onto a seperate canvas (like rsi)
3. test these
4. implement drawing tool bar.
5. implement drawing tools.
Make helper fns to easily convert to the TimeSeriesData enum so that strategies can just pass objects to the helper and it will convert based on the type of BaseDataEnum to the correct object