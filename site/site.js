const VIDEO_ID = "8EbjNOFRLiA";

// The API answers cross-origin requests and the download links don't, so the
// newest installer is found here and the button points straight at it. Until
// then, or if GitHub doesn't answer, the button opens the releases page.
async function findInstaller() {
  const button = document.getElementById("download");
  const meta = document.getElementById("download-meta");
  const timeout = new AbortController();
  const timer = setTimeout(() => timeout.abort(), 6000);

  try {
    const response = await fetch("https://api.github.com/repos/Bertrram/omoio/releases/latest", { signal: timeout.signal });
    if (!response.ok) throw new Error(`GitHub answered ${response.status}`);
    const release = await response.json();
    const installer = release.assets.find((asset) => asset.name.endsWith("_x64-setup.exe"));
    if (!installer) throw new Error("No installer in the latest release");

    button.href = installer.browser_download_url;
    const version = release.tag_name.replace(/^v/, "");
    // Mebibytes, because that is what Windows shows once the file is saved.
    const size = (installer.size / 1048576).toFixed(1);
    const notes = document.createElement("a");
    notes.href = release.html_url;
    notes.textContent = "Release notes";
    meta.replaceChildren(`Version ${version} · ${size} MB · 64-bit Windows · `, notes);
  } catch {
    meta.textContent = "Opens the releases page on GitHub";
  } finally {
    clearTimeout(timer);
  }
}

// Nothing from YouTube loads until the video is pressed.
function playVideoInPlace(event) {
  event.preventDefault();
  const frame = event.currentTarget;
  const player = document.createElement("iframe");
  player.src = `https://www.youtube-nocookie.com/embed/${VIDEO_ID}?autoplay=1&rel=0`;
  player.title = "The Omoio video";
  player.allow = "autoplay; encrypted-media; picture-in-picture; fullscreen";
  player.allowFullscreen = true;

  const box = document.createElement("div");
  box.className = "video-frame";
  box.append(player);
  frame.replaceWith(box);
  player.focus();
}

if (!/Windows/.test(navigator.userAgent)) {
  document.getElementById("elsewhere").hidden = false;
}

document.getElementById("video").addEventListener("click", playVideoInPlace);
findInstaller();
