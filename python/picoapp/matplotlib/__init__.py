import numpy as np
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.figure import Figure

from .. import Image


def figure_to_image(fig: Figure) -> Image:
    """
    Helper function to convert a matplotlib figure to an image that
    can be rendered by picoapp.
    """

    # Inspired by:
    # https://stackoverflow.com/a/62040123/1804173
    canvas = FigureCanvasAgg(fig)
    canvas.draw()
    rgb_data = np.asarray(canvas.buffer_rgba())

    return Image.from_3d_array(rgb_data)
