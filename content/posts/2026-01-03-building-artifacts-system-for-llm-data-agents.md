+++
title = "Building an Artifacts System for Our LLM Data Agents"
date = "2026-01-03"

[taxonomies]
tags=["ai", "coding"]
+++

At my day job, I've been working on an AI-powered data assistant that helps users query and analyze advertising campaign data through natural language. The initial version worked great for one-off questions, but I quickly discovered a fundamental challenge that was costing us both time and money. The AI had no memory of what data it had just fetched, unless it was stored in the context window.

## When Every Question Starts from Scratch

Consider this scenario: a user asks "Show me the spend and CPM for my campaigns during last Christmas" (spend and CPM are ad-related metrics). The assistant generates SQL, queries the data warehouse, and returns 500 rows of campaign data. Great.

Then the user asks: "Filter that to just the top 10 by spend."

Without a memory system, the assistant faced two problematic options. The first was to re-query the database entirely. This meant generating a new SQL query, hitting the data warehouse again, and potentially returning different results if the underlying data had changed, even slightly, between requests. Users expect to manipulate the same data they just saw, not a fresh snapshot.

The second option was to keep all 500 rows in the conversation context. While modern models support impressive context lengths, stuffing hundreds of database rows into that window consumes tokens rapidly, increases costs, and can degrade response quality as the model struggles to find relevant information in a sea of data.

Neither worked well. I needed something smarter.

## Store Big, Sample Small, Query on Demand

The insight that unlocked the solution was simple: the LLM doesn't need to _see_ all the data to _understand_ it. A three-row sample tells the model everything it needs to know about column names and data types. The full dataset can live elsewhere, ready to be queried when needed.

This led to the artifacts system:

1. **PostgreSQL** stores the full dataset as JSON with a unique artifact ID
2. **DuckDB** handles in-memory SQL queries against stored artifacts
3. **The LLM** only sees the artifact ID, row count, and a small sample

I store artifacts in PostgreSQL using a simple model with JSONB for the data column:

```python
class ConversationArtifact(Base):
    __tablename__ = "conversation_artifacts"

    id = Column(UUID, primary_key=True, default=uuid.uuid4)
    conversation_id = Column(String, nullable=False)
    data = Column(JSONB, nullable=False)
    created_at = Column(DateTime, server_default=func.now())
```

Why JSONB? Different queries return different columns. A spend report has different fields than a performance breakdown. JSONB lets me store any shape of data without schema migrations every time I add a new query type.

When the agent fetches data from the warehouse, I store the full result but only return a sample to the LLM. It looks something like this:

```python
def get_data(query_params: dict) -> dict:
    # Execute SQL against the data warehouse
    df = run_sql_query(generated_sql)

    # Store full dataset in PostgreSQL
    artifact = artifact_repo.insert(
        conversation_id=conversation_id,
        data=df.to_json(orient="records")
    )

    # Return only what the LLM needs
    return {
        "artifact_id": str(artifact.id),
        "sample": df.head(3).to_dict(orient="records"),
        "total_rows": len(df),
        "columns": list(df.columns)
    }
```

The LLM receives something like:

```json
{
  "artifact_id": "abc-123",
  "sample": [
    { "campaign": "Holiday Sale", "spend": 1500.0, "cpm": 2.34 },
    { "campaign": "Winter Promo", "spend": 1200.0, "cpm": 1.98 },
    { "campaign": "Year End", "spend": 980.0, "cpm": 2.1 }
  ],
  "total_rows": 500,
  "columns": ["campaign", "spend", "cpm", "impressions", "clicks"]
}
```

Three rows is enough for the model to understand the data structure. It knows there's a "spend" column with decimal values, a "campaign" column with strings, and so on. The full 500 rows stay safely in PostgreSQL.

And for the last part - this is where the magic happens. When users want to filter, aggregate, or transform their data, we can load the artifact into DuckDB and run SQL against it by exposing another tool to the agent:

```python
def execute_sql_on_dataframe(artifact_ids: list[str], query: str) -> dict:
    # Load artifacts from PostgreSQL
    artifacts = [artifact_repo.get_by_id(id) for id in artifact_ids]

    # Create ephemeral DuckDB connection (in-memory)
    conn = duckdb.connect()

    # Register each artifact as a table
    for i, artifact in enumerate(artifacts):
        df = pd.read_json(StringIO(artifact.data))
        conn.register(f"df_{i}", df)

    # Execute the SQL query
    result_df = conn.sql(query).df()

    # Store result as a new artifact
    new_artifact = artifact_repo.insert(
        conversation_id=conversation_id,
        data=result_df.to_json(orient="records")
    )

    return {
        "artifact_id": str(new_artifact.id),
        "results": result_df.to_dict(orient="records"),
        "total_rows": len(result_df)
    }
```

I chose DuckDB for a few reasons:

- **Native Pandas integration** - `conn.register()` turns a DataFrame into a queryable table with zero serialization overhead
- **Full SQL support** - Including JOINs across multiple artifacts, window functions, CTEs, everything
- **In-memory and ephemeral** - No disk management, no cleanup, connections are garbage collected automatically
- **Fast** - DuckDB is optimized for analytical queries on columnar data

The consistent table naming (`df_0`, `df_1`, etc.) is important. It makes the LLM's job predictable when generating SQL for multi-artifact queries. We instruct the LLM to use these table names in the order of the artifact IDs it receives so it can reference the correct data.

## Conversation Flow Example

Here's how a multi-turn conversation actually works:

```
User: "Show me spend by line item for last week"
  → get_data executes
  → artifact_abc created (847 rows)
  → LLM sees: 3-row sample + "847 total rows"

User: "Which ones have CTR below 0.5%?"
  → execute_sql_on_dataframe(["artifact_abc"],
      "SELECT * FROM df_0 WHERE ctr < 0.005")
  → artifact_def created (127 rows)

User: "Sum the spend for those"
  → execute_sql_on_dataframe(["artifact_def"],
      "SELECT SUM(spend) as total_spend FROM df_0")
  → Returns: $45,230

User: "Compare that to the previous week"
  → get_data for previous week → artifact_ghi
  → execute_sql_on_dataframe(["artifact_def", "artifact_ghi"],
      "SELECT 'current' as period, SUM(spend) FROM df_0
       UNION ALL
       SELECT 'previous' as period, SUM(spend) FROM df_1")
```

One database query, four analyses. The data stays consistent throughout. The user can keep drilling down, pivoting, and comparing without ever triggering another warehouse query.

## Wrapping Up

The artifacts pattern turned out to be simpler than I expected and more powerful than I hoped. The core idea is straightforward: store the full data externally, give the LLM a reference and a sample, let it request operations on demand.

This solution reduced latency, saved us money, allowed our users to query for thousands of rows, and significantly reduced LLM hallucinations.

This pattern isn't limited to data analysis. Anywhere you have large content that needs to persist across conversation turns, whether that's documents, code files, or query results, the same approach applies. Store big, sample small, query on demand.
