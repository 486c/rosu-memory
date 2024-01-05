let socket = new ReconnectingWebSocket("ws://127.0.0.1:9001/ws");

let bg = document.getElementById("bg");
let star = document.getElementById("star_span");
let pp = document.getElementById("pp");
let hun = document.getElementById("h100");
let fifty = document.getElementById("h50");
let miss = document.getElementById("h0");
let time = document.getElementById("time");

socket.onopen = () => {
	console.log("Successfully Connected");
};

socket.onclose = event => {
	console.log("Socket Closed Connection: ", event);
	socket.send("Client Closed!")
};

socket.onerror = error => {
	console.log("Socket Error: ", error);
};

let tempState;
let tempImg;
socket.onmessage = event => {
	let data = JSON.parse(event.data);
	if (tempState !== data.beatmap.paths.background_path_full) {
		tempState = data.beatmap.paths.background_path_full
		bg.setAttribute('src', `http://127.0.0.1:9001/Songs/${data.background_path_full}`)
	}
	if (data.playtime > 1000) {
		let seconds = (data.playtime/1000).toFixed(0);
		let minutes = Math.floor(seconds % 3600 / 60).toString();

		if (seconds > 60) {
			time.innerHTML = `${minutes}m ${seconds-(minutes*60)}s`
		} else {
			time.innerHTML = `${seconds}s`
		}
	}
	if (data.gameplay.current_pp != '') {
		let ppData = data.current_pp;
		pp.innerHTML = Math.round(ppData) + "pp"
	} else {
		pp.innerHTML = 0 + "pp"
	}
	if (data.stars_mods != '') {
		let SR = data.stars_mods;
		star.innerHTML = SR.toFixed(2)
	} else {
		star.innerHTML = 0
	}
	if (data.gameplay.hit_100 > 0) {
		hun.innerHTML = data.gameplay.hit_100;
	} else {
		hun.innerHTML = 0
	}
	if (data.gameplay.hit_50 > 0) {
		fifty.innerHTML = data.gameplay.hit_50;
	} else {
		fifty.innerHTML = 0
	}
	if (data.gameplay.hit_miss > 0) {
		miss.innerHTML = data.gameplay.hit_miss;
	} else {
		miss.innerHTML = 0
	}

	console.log(data.stars_mods);
}



//Received: '{"menuContainer":{"osuState":2,"bmID":1219126,"bmSetID":575767,"CS":4,"AR":9.5,"OD":8,"HP":6,"bmInfo":"BTS - Not Today [Tomorrow]","bmFolder":"575767 BTS - Not Today","pathToBM":"BTS - Not Today (DeRandom Otaku) [Tomorrow].osu","bmCurrentTime":8861,"bmMinBPM":0,"bmMaxBPM":0},"gameplayContainer":{"300":21,"100":0,"50":0,"miss":0,"accuracy":100,"score":24612,"combo":36,"gameMode":0,"appliedMods":2048,"maxCombo":36,"pp":""}}'
