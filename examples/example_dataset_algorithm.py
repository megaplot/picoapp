"""Pick a dataset and a clustering algorithm, each with its own parameters.

Each selector sits on top of its parameter column. Switching a selector back
and forth keeps every parameter's value.

The dataset and the clustering are separate `pa.memoize` nodes, so moving an
algorithm parameter re-runs only the clustering, not the dataset. That split
is an optimization: a single node computing both works just as well, it only
re-runs more.
"""

import numpy as np

import picoapp as pa

_NUM_POINTS = 400
_CANVAS_SIZE = 480
_CLUSTER_COLORS = np.array(
    [
        [230, 25, 75],
        [60, 180, 75],
        [0, 130, 200],
        [245, 130, 48],
        [145, 30, 180],
        [70, 240, 240],
        [240, 50, 230],
        [210, 245, 60],
        [250, 190, 212],
        [0, 128, 128],
    ],
    dtype=np.uint8,
)
_NOISE_COLOR = np.array([128, 128, 128], dtype=np.uint8)

dataset_select = pa.Radio("Dataset", ["blobs", "moons"])
blobs_count = pa.IntSlider("Number of blobs", 1, 3, 8)
blobs_spread = pa.Slider("Blob spread", 0.05, 0.5, 2.0, decimal_places=2)
moons_noise = pa.Slider("Moon noise", 0.0, 0.1, 0.5, decimal_places=2)

algorithm_select = pa.Radio("Algorithm", ["k-means", "DBSCAN"])
kmeans_k = pa.IntSlider("k", 1, 3, 10)
kmeans_init = pa.Radio("Initialization", ["first points", "random"])
kmeans_seed = pa.IntSlider("Seed", 0, 0, 100)  # only shown for "random"
dbscan_eps = pa.Slider("eps", 0.05, 0.3, 1.5, decimal_places=2)
dbscan_min_samples = pa.IntSlider("Min samples", 1, 5, 20)


@pa.memoize
def dataset() -> np.ndarray:
    """The points, shape (n, 2). Reads only the selected dataset's parameters."""
    print(f"dataset: {dataset_select.value}")
    rng = np.random.default_rng(0)
    if dataset_select.value == "blobs":
        centers = rng.uniform(-5.0, 5.0, size=(blobs_count.value, 2))
        labels = rng.integers(0, blobs_count.value, size=_NUM_POINTS)
        noise = rng.normal(scale=blobs_spread.value, size=(_NUM_POINTS, 2))
        return centers[labels] + noise
    t = rng.uniform(0.0, np.pi, size=_NUM_POINTS)
    upper = rng.random(_NUM_POINTS) < 0.5
    xs = np.where(upper, np.cos(t), 1.0 - np.cos(t))
    ys = np.where(upper, np.sin(t), 0.5 - np.sin(t))
    points = np.stack([xs, ys], axis=1)
    return points + rng.normal(scale=moons_noise.value, size=points.shape)


def kmeans(points: np.ndarray, k: int, seed: int | None) -> np.ndarray:
    """Cluster labels per point; `seed=None` starts from the first `k` points."""
    if seed is None:
        centers = points[:k]
    else:
        rng = np.random.default_rng(seed)
        centers = points[rng.choice(len(points), size=k, replace=False)]
    labels = np.zeros(len(points), dtype=int)
    for _ in range(50):
        distances = np.linalg.norm(points[:, None, :] - centers[None, :, :], axis=2)
        labels = np.argmin(distances, axis=1)
        new_centers = np.array(
            [
                points[labels == i].mean(axis=0) if np.any(labels == i) else centers[i]
                for i in range(k)
            ]
        )
        if np.allclose(new_centers, centers):
            break
        centers = new_centers
    return labels


def dbscan(points: np.ndarray, eps: float, min_samples: int) -> np.ndarray:
    """Cluster labels per point, -1 for noise."""
    distances = np.linalg.norm(points[:, None, :] - points[None, :, :], axis=2)
    neighbors = [np.flatnonzero(row <= eps) for row in distances]
    is_core = np.array([len(n) >= min_samples for n in neighbors])
    labels = np.full(len(points), -1)
    cluster = 0
    for start in np.flatnonzero(is_core):
        if labels[start] != -1:
            continue
        labels[start] = cluster
        queue = [start]
        while queue:
            point = queue.pop()
            if not is_core[point]:
                continue
            for neighbor in neighbors[point]:
                if labels[neighbor] == -1:
                    labels[neighbor] = cluster
                    queue.append(neighbor)
        cluster += 1
    return labels


def scatter_image(points: np.ndarray, labels: np.ndarray) -> pa.Image:
    """Draws each point as a small square, colored by its label."""
    image = np.full((_CANVAS_SIZE, _CANVAS_SIZE, 4), 255, dtype=np.uint8)
    lo, hi = points.min(axis=0), points.max(axis=0)
    scale = (_CANVAS_SIZE - 8) / np.maximum(hi - lo, 1e-9)
    pixels = ((points - lo) * scale + 4).astype(int)
    colors = np.where(
        (labels >= 0)[:, None],
        _CLUSTER_COLORS[labels % len(_CLUSTER_COLORS)],
        _NOISE_COLOR,
    )
    for (x, y), color in zip(pixels, colors):
        row = _CANVAS_SIZE - 1 - y
        image[row - 2 : row + 2, x - 2 : x + 2, :3] = color
    return pa.Image.from_3d_array(image)


@pa.memoize
def clustering() -> pa.Image:
    points = dataset()
    print(f"clustering: {algorithm_select.value}")
    if algorithm_select.value == "k-means":
        seed = kmeans_seed.value if kmeans_init.value == "random" else None
        labels = kmeans(points, kmeans_k.value, seed)
    else:
        labels = dbscan(points, dbscan_eps.value, dbscan_min_samples.value)
    return scatter_image(points, labels)


def view() -> pa.Element:
    dataset_params: list[pa.InputBase]
    if dataset_select.value == "blobs":
        dataset_params = [blobs_count, blobs_spread]
    else:
        dataset_params = [moons_noise]

    algorithm_params: list[pa.InputBase]
    if algorithm_select.value == "k-means":
        algorithm_params = [kmeans_k, kmeans_init]
        if kmeans_init.value == "random":
            algorithm_params.append(kmeans_seed)
    else:
        algorithm_params = [dbscan_eps, dbscan_min_samples]

    return pa.Row(
        pa.Column(dataset_select, *dataset_params),
        pa.Column(algorithm_select, *algorithm_params),
        clustering,
    )


pa.run(view)
