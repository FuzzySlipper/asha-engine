# GitHub Check Gates

ASHA runs the same repo gate locally and in GitHub Actions:

```bash
./harness/ci/check-all.sh
```

The workflow is `.github/workflows/offline-ci.yml`. It runs automatically on
pushes to `main`, pull requests, and manual dispatch.

For Den Review GitHub check gates, use:

```json
{
  "project_id": "asha",
  "repository": "FuzzySlipper/asha",
  "ref": "main",
  "required_checks": ["Verify ASHA"]
}
```

Agents should register the exact pushed commit SHA after a task commit is
pushed. The Den service records pass, fail, timeout, or superseded evidence on
the task thread; GitHub Actions remains the runner.
