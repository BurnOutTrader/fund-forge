### Problems 
The static buffer is an issue, some events don't need to be buffered and cause deadlock when added after warm up
This issue is directly related to warming up subscriptions and indicators, somehow adding a buffer event to the public static buffer causes Mutex dead lock...
 - subscription events in subscription handler
 - Possibly indicator events