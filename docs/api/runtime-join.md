# RuntimeJoin API notes

RuntimeJoin is configured inside `update` operation arguments.

## Example

```json
{
  "joins": [
    {
      "alias": "fx",
      "dataset_id": "550e8400-e29b-41d4-a716-446655440000",
      "on": {
        "source": "currency = fx.from_currency AND fx.to_currency = 'USD'"
      }
    }
  ],
  "assignments": []
}
```

## Resolution behavior

- Resolver precedence: project override -> dataset resolver_id -> system default.
- `dataset_version` omitted -> latest active dataset version.
- `dataset_version` set -> exact pinned version.
- Bitemporal joins apply asOf filtering using run period start date.
