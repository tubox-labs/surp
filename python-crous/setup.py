import warnings

from setuptools import setup

# setup message
warnings.warn(
    "Package 'crous' has moved to 'surp'. Install with: pip install surp",
    DeprecationWarning,
)

setup()
