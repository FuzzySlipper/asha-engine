const DATE_PATTERN = /^\d{4}-\d{2}-\d{2}$/u;
const TASK_REF_PATTERN = /^#\d+$/u;
const EXEMPTION_KINDS = ['fileLineExemptions', 'rootBarrelExemptions'];

function isRecord(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function isPositiveInteger(value) {
  return typeof value === 'number' && Number.isSafeInteger(value) && value > 0;
}

function isIsoDate(value) {
  if (typeof value !== 'string' || !DATE_PATTERN.test(value)) {
    return false;
  }
  const parsed = new Date(`${value}T00:00:00Z`);
  return !Number.isNaN(parsed.valueOf()) && parsed.toISOString().slice(0, 10) === value;
}

export function validateExemptionMetadata(kind, rel, value, failures, today = undefined) {
  if (!isRecord(value)) {
    return;
  }
  if (typeof value.owner !== 'string' || value.owner.trim().length < 3) {
    failures.push(`FAIL: ${rel} ${kind}.owner must name the accountable lane or cell.`);
  }
  if (typeof value.rationale !== 'string' || value.rationale.trim().length < 30) {
    failures.push(`FAIL: ${rel} ${kind}.rationale must explain the structural exception.`);
  }
  if (typeof value.introducedBy !== 'string' || !TASK_REF_PATTERN.test(value.introducedBy)) {
    failures.push(`FAIL: ${rel} ${kind}.introducedBy must be a Den task ref like #5761.`);
  }
  if (!isIsoDate(value.reviewBy)) {
    failures.push(`FAIL: ${rel} ${kind}.reviewBy must be an ISO date.`);
  } else if (today !== undefined && value.reviewBy < today) {
    failures.push(
      `FAIL: ${rel} ${kind} review metadata expired on ${value.reviewBy}; ` +
        'renew the review or remove the exemption.',
    );
  }
  if (!isPositiveInteger(value.warningLines) || value.warningLines >= value.maxLines) {
    failures.push(`FAIL: ${rel} ${kind}.warningLines must be a positive integer below maxLines.`);
  }
  if (typeof value.reviewTrigger !== 'string' || value.reviewTrigger.trim().length < 30) {
    failures.push(`FAIL: ${rel} ${kind}.reviewTrigger must name concrete review triggers.`);
  }
  if (typeof value.removalCondition !== 'string' || value.removalCondition.trim().length < 30) {
    failures.push(`FAIL: ${rel} ${kind}.removalCondition must define when the exception is removed.`);
  }
}

export function validateBaselineChange(kind, rel, value, failures) {
  if (value === undefined) {
    return undefined;
  }
  if (!isRecord(value)) {
    failures.push(`FAIL: ${rel} ${kind} baselineChange must be an object.`);
    return undefined;
  }

  if (!isIsoDate(value.changedAt)) {
    failures.push(`FAIL: ${rel} ${kind} baselineChange.changedAt must be an ISO date.`);
  }
  if (typeof value.changeTask !== 'string' || !TASK_REF_PATTERN.test(value.changeTask)) {
    failures.push(`FAIL: ${rel} ${kind} baselineChange.changeTask must be a Den task ref like #5505.`);
  }
  if (typeof value.reason !== 'string' || value.reason.trim().length < 30) {
    failures.push(`FAIL: ${rel} ${kind} baselineChange.reason must explain the temporary raise.`);
  }
  if (value.previousMaxLines !== null && !isPositiveInteger(value.previousMaxLines)) {
    failures.push(
      `FAIL: ${rel} ${kind} baselineChange.previousMaxLines must be null or a positive integer.`,
    );
  }
  if (!isPositiveInteger(value.newMaxLines)) {
    failures.push(`FAIL: ${rel} ${kind} baselineChange.newMaxLines must be a positive integer.`);
  }

  return value;
}

export function auditTsSourceShapePolicy(basePolicy, currentPolicy, failures) {
  auditSourceShapePolicy(
    'TypeScript',
    EXEMPTION_KINDS,
    basePolicy,
    currentPolicy,
    failures,
  );
}

export function auditRustSourceShapePolicy(basePolicy, currentPolicy, failures) {
  auditSourceShapePolicy(
    'Rust',
    ['fileLineExemptions'],
    basePolicy,
    currentPolicy,
    failures,
  );
}

function auditSourceShapePolicy(language, exemptionKinds, basePolicy, currentPolicy, failures) {
  if (currentPolicy.maxSourceLines > basePolicy.maxSourceLines) {
    failures.push(
      `FAIL: global ${language} source cap increased from ${basePolicy.maxSourceLines} to ` +
        `${currentPolicy.maxSourceLines}; split source files instead of raising maxSourceLines.`,
    );
  }

  for (const kind of exemptionKinds) {
    const baseEntries = isRecord(basePolicy[kind]) ? basePolicy[kind] : {};
    const currentEntries = isRecord(currentPolicy[kind]) ? currentPolicy[kind] : {};
    for (const [rel, currentEntry] of Object.entries(currentEntries)) {
      if (!isRecord(currentEntry) || !isPositiveInteger(currentEntry.maxLines)) {
        continue;
      }
      const baseEntry = isRecord(baseEntries[rel]) ? baseEntries[rel] : undefined;
      const previousMaxLines = baseEntry?.maxLines;
      const isNew = baseEntry === undefined;
      const isRaised = isPositiveInteger(previousMaxLines) && currentEntry.maxLines > previousMaxLines;
      if (!isNew && !isRaised) {
        continue;
      }

      const change = validateBaselineChange(kind, rel, currentEntry.baselineChange, failures);
      if (change === undefined) {
        const action = isNew ? 'new exemption' : 'baseline increase';
        failures.push(
          `FAIL: ${rel} ${kind} ${action} requires baselineChange audit metadata with ` +
            'changedAt, changeTask, reason, previousMaxLines, and newMaxLines.',
        );
        continue;
      }

      const expectedPreviousMaxLines = isNew ? null : previousMaxLines;
      if (change.previousMaxLines !== expectedPreviousMaxLines) {
        failures.push(
          `FAIL: ${rel} ${kind} baselineChange.previousMaxLines must equal ` +
            `${String(expectedPreviousMaxLines)} for this policy diff.`,
        );
      }
      if (change.newMaxLines !== currentEntry.maxLines) {
        failures.push(
          `FAIL: ${rel} ${kind} baselineChange.newMaxLines must equal ` +
            `${currentEntry.maxLines} for this policy diff.`,
        );
      }
    }
  }
}
