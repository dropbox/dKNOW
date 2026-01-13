"""
Core PDFProcessor implementation.

This module provides Python bindings for the pdfium_cli binary,
wrapping subprocess calls in a clean, Pythonic API.
"""

import json
import subprocess
import tempfile
from pathlib import Path
from typing import Optional, Union, List, Dict, Any


class PDFError(Exception):
    """Base exception for PDF processing errors."""
    pass


class PDFProcessor:
    """
    High-performance PDF processor using PDFium.

    This class provides a Python interface to the optimized pdfium_cli binary,
    supporting text extraction, image rendering, and metadata extraction.

    Features:
        - Multi-process parallelism (up to 16 workers)
        - JPEG fast path for scanned PDFs (545x speedup)
        - Adaptive threading for image rendering
        - Page range selection
        - Batch processing support

    Attributes:
        cli_path: Path to the pdfium_cli binary.
        default_workers: Default number of worker processes.
        debug: Whether to enable debug mode.

    Example:
        >>> processor = PDFProcessor()
        >>> text = processor.extract_text("document.pdf")
        >>> processor.render_pages("document.pdf", "output/", workers=4)
    """

    def __init__(
        self,
        cli_path: Optional[Union[str, Path]] = None,
        default_workers: int = 1,
        debug: bool = False
    ):
        """
        Initialize PDF processor.

        Args:
            cli_path: Path to pdfium_cli binary. If None, searches in default locations.
            default_workers: Default number of worker processes (1-16).
            debug: Enable debug mode with detailed tracing.

        Raises:
            PDFError: If pdfium_cli binary cannot be found.
        """
        self.cli_path = self._find_cli_binary(cli_path)
        self.default_workers = max(1, min(16, default_workers))
        self.debug = debug

    def _find_cli_binary(self, cli_path: Optional[Union[str, Path]] = None) -> Path:
        """
        Find pdfium_cli binary.

        Search order:
        1. Provided cli_path
        2. out/Release/pdfium_cli (relative to package)
        3. System PATH

        Args:
            cli_path: Optional explicit path to binary.

        Returns:
            Path to pdfium_cli binary.

        Raises:
            PDFError: If binary cannot be found.
        """
        if cli_path:
            path = Path(cli_path)
            if path.exists() and path.is_file():
                return path
            raise PDFError(f"Specified pdfium_cli not found: {cli_path}")

        # Try relative to this file (assumes package is in pdfium_fast repo)
        package_root = Path(__file__).parent.parent.parent
        release_binary = package_root / "out" / "Release" / "pdfium_cli"
        if release_binary.exists():
            return release_binary

        # Try system PATH
        import shutil
        system_binary = shutil.which("pdfium_cli")
        if system_binary:
            return Path(system_binary)

        raise PDFError(
            "pdfium_cli binary not found. "
            "Please specify cli_path or ensure pdfium_cli is in PATH."
        )

    def _run_command(
        self,
        args: List[str],
        timeout: Optional[int] = None,
        check: bool = True
    ) -> subprocess.CompletedProcess:
        """
        Run pdfium_cli command.

        Args:
            args: Command arguments (excluding binary path).
            timeout: Command timeout in seconds.
            check: Whether to raise exception on non-zero exit code.

        Returns:
            CompletedProcess instance.

        Raises:
            PDFError: If command fails and check=True.
        """
        cmd = [str(self.cli_path)] + args

        if self.debug:
            cmd.insert(1, "--debug")

        try:
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=timeout,
                check=False
            )

            if check and result.returncode != 0:
                error_msg = result.stderr.strip() or result.stdout.strip()
                raise PDFError(f"Command failed: {error_msg}")

            return result

        except subprocess.TimeoutExpired:
            raise PDFError(f"Command timed out after {timeout} seconds")
        except FileNotFoundError:
            raise PDFError(f"Binary not found: {self.cli_path}")
        except Exception as e:
            raise PDFError(f"Command execution failed: {str(e)}")

    def extract_text(
        self,
        pdf_path: Union[str, Path],
        output_path: Optional[Union[str, Path]] = None,
        workers: Optional[int] = None,
        pages: Optional[Union[int, tuple]] = None,
        timeout: Optional[int] = None
    ) -> str:
        """
        Extract text from PDF.

        Args:
            pdf_path: Path to input PDF file.
            output_path: Path to output text file. If None, returns text directly.
            workers: Number of worker processes (1-16). Default uses self.default_workers.
            pages: Page selection. Can be:
                - int: Single page (e.g., 5)
                - tuple: Page range (e.g., (1, 10))
            timeout: Command timeout in seconds.

        Returns:
            Extracted text as string (if output_path is None).

        Raises:
            PDFError: If extraction fails.

        Example:
            >>> # Extract all text
            >>> text = processor.extract_text("document.pdf")
            >>>
            >>> # Extract with 4 workers
            >>> text = processor.extract_text("document.pdf", workers=4)
            >>>
            >>> # Extract specific page range
            >>> text = processor.extract_text("document.pdf", pages=(1, 10))
            >>>
            >>> # Save to file
            >>> processor.extract_text("document.pdf", "output.txt")
        """
        pdf_path = Path(pdf_path)
        if not pdf_path.exists():
            raise PDFError(f"PDF file not found: {pdf_path}")

        # Use temporary file if no output path specified
        use_temp = output_path is None
        if use_temp:
            temp_file = tempfile.NamedTemporaryFile(mode='w', delete=False, suffix='.txt')
            output_path = Path(temp_file.name)
            temp_file.close()
        else:
            output_path = Path(output_path)

        try:
            # Build command
            args = []

            # Workers
            num_workers = workers if workers is not None else self.default_workers
            if num_workers > 1:
                args.extend(["--workers", str(num_workers)])

            # Page selection
            if pages is not None:
                if isinstance(pages, int):
                    args.extend(["--pages", str(pages)])
                elif isinstance(pages, tuple) and len(pages) == 2:
                    args.extend(["--pages", f"{pages[0]}-{pages[1]}"])
                else:
                    raise PDFError(f"Invalid pages argument: {pages}")

            # Operation and paths
            args.extend(["extract-text", str(pdf_path), str(output_path)])

            # Execute
            self._run_command(args, timeout=timeout)

            # Read result if temporary file
            if use_temp:
                # pdfium_cli outputs UTF-32 LE format
                with open(output_path, 'r', encoding='utf-32-le') as f:
                    return f.read()
            else:
                return ""

        finally:
            # Clean up temporary file
            if use_temp and output_path.exists():
                output_path.unlink()

    def extract_jsonl(
        self,
        pdf_path: Union[str, Path],
        page: int = 0,
        output_path: Optional[Union[str, Path]] = None,
        timeout: Optional[int] = None
    ) -> Dict[str, Any]:
        """
        Extract text with rich metadata in JSONL format.

        Extracts character positions, bounding boxes, font metadata, and more.
        Note: Currently supports single-page extraction only.

        Args:
            pdf_path: Path to input PDF file.
            page: Page number to extract (0-indexed).
            output_path: Path to output JSONL file. If None, returns parsed JSON.
            timeout: Command timeout in seconds.

        Returns:
            Parsed JSONL data as dictionary (if output_path is None).

        Raises:
            PDFError: If extraction fails.

        Example:
            >>> # Extract metadata for first page
            >>> metadata = processor.extract_jsonl("document.pdf", page=0)
            >>> print(metadata['char_count'])
            >>>
            >>> # Save to file
            >>> processor.extract_jsonl("document.pdf", page=0, output_path="meta.jsonl")
        """
        pdf_path = Path(pdf_path)
        if not pdf_path.exists():
            raise PDFError(f"PDF file not found: {pdf_path}")

        # Use temporary file if no output path specified
        use_temp = output_path is None
        if use_temp:
            temp_file = tempfile.NamedTemporaryFile(mode='w', delete=False, suffix='.jsonl')
            output_path = Path(temp_file.name)
            temp_file.close()
        else:
            output_path = Path(output_path)

        try:
            # Build command
            args = [
                "--pages", str(page),
                "extract-jsonl",
                str(pdf_path),
                str(output_path)
            ]

            # Execute
            self._run_command(args, timeout=timeout)

            # Read and parse result if temporary file
            if use_temp:
                with open(output_path, 'r', encoding='utf-8') as f:
                    # JSONL format: one JSON object per line
                    # Since we request a single page, read first non-empty line
                    for line in f:
                        line = line.strip()
                        if line:
                            return json.loads(line)
                    return {}  # No data
            else:
                return {}

        finally:
            # Clean up temporary file
            if use_temp and output_path.exists():
                output_path.unlink()

    def render_pages(
        self,
        pdf_path: Union[str, Path],
        output_dir: Union[str, Path],
        workers: Optional[int] = None,
        pages: Optional[Union[int, tuple]] = None,
        format: str = "png",
        jpeg_quality: int = 90,
        adaptive: bool = False,
        timeout: Optional[int] = None
    ) -> List[Path]:
        """
        Render PDF pages to images.

        Args:
            pdf_path: Path to input PDF file.
            output_dir: Directory for output images.
            workers: Number of worker processes (1-16). Default uses self.default_workers.
            pages: Page selection. Can be:
                - int: Single page (e.g., 5)
                - tuple: Page range (e.g., (1, 10))
            format: Output format: "png", "jpg", "jpeg", or "ppm".
            jpeg_quality: JPEG quality (0-100, only for JPEG format).
            adaptive: Enable adaptive threading (auto-selects thread count).
            timeout: Command timeout in seconds.

        Returns:
            List of paths to generated image files.

        Raises:
            PDFError: If rendering fails.

        Example:
            >>> # Render all pages to PNG
            >>> images = processor.render_pages("document.pdf", "output/")
            >>>
            >>> # Render with 4 workers and adaptive threading
            >>> images = processor.render_pages(
            ...     "document.pdf", "output/",
            ...     workers=4, adaptive=True
            ... )
            >>>
            >>> # Render specific pages to JPEG
            >>> images = processor.render_pages(
            ...     "document.pdf", "output/",
            ...     pages=(1, 10), format="jpg", jpeg_quality=95
            ... )
        """
        pdf_path = Path(pdf_path)
        if not pdf_path.exists():
            raise PDFError(f"PDF file not found: {pdf_path}")

        output_dir = Path(output_dir)
        output_dir.mkdir(parents=True, exist_ok=True)

        # Build command
        args = []

        # Workers
        num_workers = workers if workers is not None else self.default_workers
        if num_workers > 1:
            args.extend(["--workers", str(num_workers)])

        # Adaptive threading
        if adaptive:
            args.append("--adaptive")

        # Page selection
        if pages is not None:
            if isinstance(pages, int):
                args.extend(["--pages", str(pages)])
            elif isinstance(pages, tuple) and len(pages) == 2:
                args.extend(["--pages", f"{pages[0]}-{pages[1]}"])
            else:
                raise PDFError(f"Invalid pages argument: {pages}")

        # Format
        if format not in ["png", "jpg", "jpeg", "ppm"]:
            raise PDFError(f"Invalid format: {format}")
        args.extend(["--format", format])

        # JPEG quality
        if format in ["jpg", "jpeg"]:
            args.extend(["--jpeg-quality", str(jpeg_quality)])

        # Operation and paths
        args.extend(["render-pages", str(pdf_path), str(output_dir) + "/"])

        # Execute
        self._run_command(args, timeout=timeout)

        # Find generated images
        if format == "jpeg":
            format = "jpg"

        image_files = sorted(output_dir.glob(f"*.{format}"))
        return image_files

    def batch_extract_text(
        self,
        input_dir: Union[str, Path],
        output_dir: Union[str, Path],
        workers: Optional[int] = None,
        pattern: str = "*.pdf",
        recursive: bool = False,
        timeout: Optional[int] = None
    ) -> None:
        """
        Batch extract text from multiple PDFs.

        Args:
            input_dir: Directory containing PDF files.
            output_dir: Directory for output text files.
            workers: Number of worker processes per PDF.
            pattern: File pattern (e.g., "*.pdf", "report_*.pdf").
            recursive: Search subdirectories recursively.
            timeout: Command timeout in seconds.

        Raises:
            PDFError: If batch extraction fails.

        Example:
            >>> # Extract all PDFs in directory
            >>> processor.batch_extract_text("pdfs/", "output/")
            >>>
            >>> # Extract with pattern and recursion
            >>> processor.batch_extract_text(
            ...     "archive/", "output/",
            ...     pattern="report_*.pdf",
            ...     recursive=True
            ... )
        """
        input_dir = Path(input_dir)
        output_dir = Path(output_dir)

        if not input_dir.exists():
            raise PDFError(f"Input directory not found: {input_dir}")

        output_dir.mkdir(parents=True, exist_ok=True)

        # Build command
        args = ["--batch"]

        # Workers
        num_workers = workers if workers is not None else self.default_workers
        if num_workers > 1:
            args.extend(["--workers", str(num_workers)])

        # Pattern
        if pattern != "*.pdf":
            args.extend(["--pattern", pattern])

        # Recursive
        if recursive:
            args.append("--recursive")

        # Operation and paths
        args.extend(["extract-text", str(input_dir), str(output_dir)])

        # Execute
        self._run_command(args, timeout=timeout)

    def batch_render_pages(
        self,
        input_dir: Union[str, Path],
        output_dir: Union[str, Path],
        workers: Optional[int] = None,
        pattern: str = "*.pdf",
        recursive: bool = False,
        format: str = "png",
        jpeg_quality: int = 90,
        adaptive: bool = False,
        timeout: Optional[int] = None
    ) -> None:
        """
        Batch render pages from multiple PDFs.

        Args:
            input_dir: Directory containing PDF files.
            output_dir: Directory for output images.
            workers: Number of worker processes per PDF.
            pattern: File pattern (e.g., "*.pdf", "report_*.pdf").
            recursive: Search subdirectories recursively.
            format: Output format: "png", "jpg", "jpeg", or "ppm".
            jpeg_quality: JPEG quality (0-100, only for JPEG format).
            adaptive: Enable adaptive threading.
            timeout: Command timeout in seconds.

        Raises:
            PDFError: If batch rendering fails.

        Example:
            >>> # Render all PDFs in directory
            >>> processor.batch_render_pages("pdfs/", "images/")
            >>>
            >>> # Batch render with options
            >>> processor.batch_render_pages(
            ...     "archive/", "images/",
            ...     workers=4,
            ...     format="jpg",
            ...     adaptive=True,
            ...     recursive=True
            ... )
        """
        input_dir = Path(input_dir)
        output_dir = Path(output_dir)

        if not input_dir.exists():
            raise PDFError(f"Input directory not found: {input_dir}")

        output_dir.mkdir(parents=True, exist_ok=True)

        # Build command
        args = ["--batch"]

        # Workers
        num_workers = workers if workers is not None else self.default_workers
        if num_workers > 1:
            args.extend(["--workers", str(num_workers)])

        # Adaptive threading
        if adaptive:
            args.append("--adaptive")

        # Pattern
        if pattern != "*.pdf":
            args.extend(["--pattern", pattern])

        # Recursive
        if recursive:
            args.append("--recursive")

        # Format
        if format not in ["png", "jpg", "jpeg", "ppm"]:
            raise PDFError(f"Invalid format: {format}")
        args.extend(["--format", format])

        # JPEG quality
        if format in ["jpg", "jpeg"]:
            args.extend(["--jpeg-quality", str(jpeg_quality)])

        # Operation and paths
        args.extend(["render-pages", str(input_dir), str(output_dir)])

        # Execute
        self._run_command(args, timeout=timeout)
