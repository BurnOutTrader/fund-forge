
## Subscription handling
- Make indicator subscribe and remove return a result directly to calling fns instead of adding to buffer.
- Live subscription in symbol subscription handler could be simplified by just allowing the data server to determine the correct primary resolution.
- Live warm up should be handled by the data server so we have the latest bars regardless of serialized history...
