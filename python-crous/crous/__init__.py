import warnings

# setup message
warnings.warn(
    "Package 'crous' has moved to 'surp'. Install with: pip install surp",
    DeprecationWarning,
    stacklevel=2,
)

from surp import *  # noqa: F401,F403
from surp import __all__ as __all__
from surp import __version__ as __version__
