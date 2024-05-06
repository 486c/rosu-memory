let socket = new ReconnectingWebSocket("ws://127.0.0.1:24050/rws");
let wrapper = document.getElementById('wrapper');
let ifFcpp = document.getElementsByClassName('ifFcpp')[0];

socket.onopen = () => console.log("Successfully Connected");
socket.onclose = event => {
    console.log("Socket Closed Connection: ", event);
    socket.send("Client Closed!");
};
socket.onerror = error => console.log("Socket Error: ", error);

let animation = {
    pp: new CountUp('pp', 0, 0, 0, 0.5, { decimalPlaces: 2, useEasing: true, useGrouping: false, separator: " ", decimal: "." }),
    ifFcpp: new CountUp('ifFcpp', 0, 0, 0, 0.5, { decimalPlaces: 2, useEasing: true, useGrouping: false, separator: " ", decimal: "." }),
    hun: new CountUp('hun', 0, 0, 0, 0.5, { decimalPlaces: 2, useEasing: true, useGrouping: false, separator: " ", decimal: "." }),
    fiv: new CountUp('fiv', 0, 0, 0, 0.5, { decimalPlaces: 2, useEasing: true, useGrouping: false, separator: " ", decimal: "." }),
    miss: new CountUp('miss', 0, 0, 0, 0.5, { decimalPlaces: 2, useEasing: true, useGrouping: false, separator: " ", decimal: "." }),
};

let tempState

socket.onmessage = event => {
    let data = JSON.parse(event.data)

    if (data.state !== tempState) {
        tempState = data.state
        if (tempState !== 2) {
            wrapper.style.transform = "translateX(-110%)"
        } else {
            wrapper.style.transform = "translateX(0)"
        }

    }
    if (data.current_pp !== '' && data.current_pp !== 0) {
        animation.pp.update(data.current_pp)
    } else {
        animation.pp.update(0)
    }
    if (data.fc_pp !== '' && data.fc_pp !== 0) {
        animation.ifFcpp.update(data.fc_pp)
    } else {
        animation.ifFcpp.update(0)
    }
    if (data.gameplay.hit_100 > 0) {
        animation.hun.update(data.gameplay.hit_100)
    } else {
        animation.hun.update(0)
    }
    if (data.gameplay.hit_50 > 0) {
        animation.fiv.update(data.gameplay.hit_50)
    } else {
        animation.fiv.update(0)
    }
    if (data.gameplay.hit_miss > 0) {
        animation.miss.update(data.gameplay.hit_miss)
    } else {
        animation.miss.update(0)
    }

    if (data.gameplay.hit_miss > 0  ||  data.gameplay.slider_breaks > 0) {
        ifFcpp.style.opacity = 1
    } else {
        ifFcpp.style.opacity = 0
    }
}
