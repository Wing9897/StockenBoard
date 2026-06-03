import { describe, it, expect } from 'vitest';
import { getGridClass } from './viewUtils';

// Characterization tests for the view-mode → CSS class mapping used by App's
// dashboard grid. Pure function, exhaustive over the ViewMode union.

describe('getGridClass', () => {
  it('maps compact to the compact asset grid', () => {
    expect(getGridClass('compact')).toBe('asset-grid compact');
  });

  it('maps list to the asset list layout', () => {
    expect(getGridClass('list')).toBe('asset-list');
  });

  it('maps grid (default) to the plain asset grid', () => {
    expect(getGridClass('grid')).toBe('asset-grid');
  });
});
