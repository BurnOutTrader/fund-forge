### Problems 
The static buffer is an issue, some events don't need to be buffered and cause deadlock when added after warm up
 - subscription events in subscription handler
 - Possibly indicator events