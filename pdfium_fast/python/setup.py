"""
Setup script for dash-pdf-extraction.
"""

from setuptools import setup, find_packages
from pathlib import Path

# Read version
version = {}
with open("dash_pdf_extraction/version.py") as f:
    exec(f.read(), version)

# Read README
readme_path = Path(__file__).parent.parent / "README.md"
long_description = ""
if readme_path.exists():
    with open(readme_path, encoding="utf-8") as f:
        long_description = f.read()

setup(
    name="dash-pdf-extraction",
    version=version["__version__"],
    description="Fast, multi-threaded PDF text extraction and rendering using PDFium",
    long_description=long_description,
    long_description_content_type="text/markdown",
    author="Andrew Yates",
    author_email="",
    url="https://github.com/yourusername/pdfium_fast",
    packages=find_packages(),
    python_requires=">=3.8",
    install_requires=[
        # No dependencies - pure Python subprocess wrapper
    ],
    extras_require={
        "dev": [
            "pytest>=7.0.0",
            "pytest-cov>=4.0.0",
            "black>=22.0.0",
            "mypy>=0.990",
        ]
    },
    classifiers=[
        "Development Status :: 4 - Beta",
        "Intended Audience :: Developers",
        "License :: OSI Approved :: Apache Software License",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
        "Programming Language :: Python :: 3.12",
        "Topic :: Software Development :: Libraries :: Python Modules",
        "Topic :: Text Processing",
    ],
    keywords="pdf extraction pdfium text-extraction image-rendering",
    project_urls={
        "Bug Reports": "https://github.com/yourusername/pdfium_fast/issues",
        "Source": "https://github.com/yourusername/pdfium_fast",
    },
)
