var Icons = {
  music: String.fromCodePoint(0xf075a),
  musicOutline: String.fromCodePoint(0xf075b),
  heartFilled: String.fromCodePoint(0xf004),
  heartOutline: String.fromCodePoint(0xf08a),
  prev: String.fromCodePoint(0xf04ae),
  play: String.fromCodePoint(0xf040a),
  pause: String.fromCodePoint(0xf03e4),
  next: String.fromCodePoint(0xf04ad),
  volume: String.fromCodePoint(0xf055f),
  vibe: String.fromCodePoint(0xf043c),
  energy: String.fromCodePoint(0xf046e),
  relax: String.fromCodePoint(0xf0704),
  work: String.fromCodePoint(0xf00f7),
  sparkles: String.fromCodePoint(0xf0e07),
  filter: String.fromCodePoint(0xf0232),
  tune: String.fromCodePoint(0xf062e),
  chevronDown: String.fromCodePoint(0xf078),
  chevronRight: String.fromCodePoint(0xf054),
  chevronLeft: String.fromCodePoint(0xf053),
  arrowLeft: String.fromCodePoint(0xf060),
  gear: String.fromCodePoint(0xf013),
  check: String.fromCodePoint(0xf00c),
  toggleOn: String.fromCodePoint(0xf205),
  toggleOff: String.fromCodePoint(0xf204),
};

var CrossfadeDurationOptions = [
  { id: 1, label: "1s" },
  { id: 2, label: "2s" },
  { id: 3, label: "3s" },
  { id: 4, label: "4s" },
  { id: 5, label: "5s" },
];

var DiversityOptions = [
  { id: "default", label: "Favorite" },
  { id: "discover", label: "Discover" },
  { id: "popular", label: "Popular" },
];

var MoodOptions = [
  { id: "all", label: "All" },
  { id: "active", label: "Active" },
  { id: "fun", label: "Fun" },
  { id: "calm", label: "Calm" },
  { id: "sad", label: "Sad" },
];

var LanguageOptions = [
  { id: "any", label: "Any" },
  { id: "not-russian", label: "Foreign" },
  { id: "russian", label: "Russian" },
  { id: "without-words", label: "No words" },
];

var StationOptions = [
  { id: "user:onyourwave", label: "My Vibe", icon: Icons.vibe },
  { id: "activity:energy", label: "Energy", icon: Icons.energy },
  { id: "mood:calm", label: "Relax", icon: Icons.relax },
  { id: "activity:work", label: "Work", icon: Icons.work },
];

function formatTime(ms) {
  if (!ms || ms <= 0) return "0:00";
  var totalSec = Math.floor(ms / 1000);
  var min = Math.floor(totalSec / 60);
  var sec = totalSec % 60;
  return min + ":" + (sec < 10 ? "0" : "") + sec;
}

function parseState(raw) {
  if (!raw) return null;
  try {
    var data = typeof raw === "string" ? JSON.parse(raw) : raw;
    if (data.state) return data.state;
    if (data.status && data.status !== "ok" && data.status !== "error")
      return data;
    return null;
  } catch (e) {
    return null;
  }
}

function makeCommand(name, payload) {
  var cmd = Object.assign({ command: name }, payload || {});
  return JSON.stringify(cmd) + "\n";
}
