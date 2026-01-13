## API Rate Limits

Configure rate limiting in your application:

```
const config = { rateLimit: { windowMs: 900000, max: 100 } };
```

## Tier Limits:

| Tier   | Requests/Min   | Daily   |
|--------|----------------|---------|
| Free   | 60             | 1,000   |
| Basic  | 600            | 10,000  |
| Pro    | 6,000          | 100,000 |