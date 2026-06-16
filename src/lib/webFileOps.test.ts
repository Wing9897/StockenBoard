import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import fc from 'fast-check';

// Feature: web-mode-file-operations, Property 5: Import file content round-trip

vi.mock('./filePicker');

describe('webFileOps', () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  it('Property 5: Import file content round-trip', async () => {
    // **Validates: Requirements 4.2, 4.3**
    const { pickFileOrThrow } = await import('./filePicker');
    const { webImportFile } = await import('./webFileOps');

    const mockedPickFileOrThrow = vi.mocked(pickFileOrThrow);

    await fc.assert(
      fc.asyncProperty(
        fc.string({ unit: 'grapheme-composite' }),
        async (content) => {
          // Create a File object from the generated string
          const file = new File([content], 'test.json', { type: 'application/json' });

          // Mock pickFileOrThrow to return the constructed File
          mockedPickFileOrThrow.mockResolvedValue(file);

          // Call webImportFile and verify round-trip
          const result = await webImportFile();

          expect(result).toBe(content);
        }
      ),
      { numRuns: 100 }
    );
  });

  // Feature: web-mode-file-operations, Property 4: Export file makes no HTTP request
  describe('Property 4: Export file makes no HTTP request', () => {
    let fetchSpy: ReturnType<typeof vi.fn>;

    beforeEach(() => {
      // Mock fetch globally to detect any HTTP calls
      fetchSpy = vi.fn();
      vi.stubGlobal('fetch', fetchSpy);

      // Mock URL.createObjectURL and URL.revokeObjectURL without replacing URL constructor
      vi.spyOn(URL, 'createObjectURL').mockReturnValue('blob:mock-url');
      vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => {});

      // Mock DOM methods used by downloadBlob
      const mockAnchor = {
        href: '',
        download: '',
        style: { display: '' },
        click: vi.fn(),
      };
      vi.spyOn(document, 'createElement').mockReturnValue(mockAnchor as unknown as HTMLElement);
      vi.spyOn(document.body, 'appendChild').mockImplementation((node) => node);
      vi.spyOn(document.body, 'removeChild').mockImplementation((node) => node);
    });

    afterEach(() => {
      vi.restoreAllMocks();
      vi.unstubAllGlobals();
    });

    // **Validates: Requirements 3.4, 3.5**
    it('webExportFile never calls fetch for any filename and content', async () => {
      const { webExportFile } = await import('./webFileOps');

      await fc.assert(
        fc.asyncProperty(
          fc.string({ minLength: 1, maxLength: 100 }),
          fc.string({ minLength: 0, maxLength: 500 }),
          async (filename, content) => {
            fetchSpy.mockClear();

            await webExportFile({ filename, content });

            expect(fetchSpy).not.toHaveBeenCalled();
          }
        ),
        { numRuns: 100 }
      );
    });
  });
});
