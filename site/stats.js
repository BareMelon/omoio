// GitHub counts each download of a release file, and the API that serves those
// counts answers cross-origin requests, so the numbers come straight from it.
async function showDownloads() {
  const total = document.getElementById("total");
  const updated = document.getElementById("updated");
  const table = document.getElementById("releases");
  const rows = table.querySelector("tbody");

  updated.textContent = "Asking GitHub";
  try {
    const response = await fetch("https://api.github.com/repos/Bertrram/omoio/releases?per_page=100");
    if (!response.ok) throw new Error(`GitHub answered ${response.status}`);
    const releases = await response.json();

    let sum = 0;
    rows.replaceChildren();
    for (const release of releases) {
      const installer = release.assets.find((asset) => asset.name.endsWith("_x64-setup.exe"));
      const count = installer ? installer.download_count : 0;
      sum += count;

      const row = document.createElement("tr");
      const date = new Date(release.published_at).toLocaleDateString(undefined, { day: "numeric", month: "short" });
      for (const text of [release.tag_name.replace(/^v/, ""), date, count.toLocaleString()]) {
        const cell = document.createElement("td");
        cell.textContent = text;
        row.append(cell);
      }
      rows.append(row);
    }

    total.textContent = sum.toLocaleString();
    table.hidden = false;
    updated.textContent = `Checked at ${new Date().toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" })}`;
  } catch {
    updated.textContent = "GitHub didn't answer. It allows 60 checks an hour from one network, so try again in a few minutes.";
  }
}

document.getElementById("refresh").addEventListener("click", showDownloads);
showDownloads();
