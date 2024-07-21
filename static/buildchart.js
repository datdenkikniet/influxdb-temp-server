import { co2_map } from "./co2_level_mapping.js"

const ctx = document.querySelector('#chart')
const load_time = document.querySelector('#load-time')
const points_loaded = document.querySelector('#points-loaded')
const timespan = document.querySelector('#timespan')
const error_message = document.querySelector("#error")
const reset = document.querySelector("#reset")
const start_date = document.querySelector("#start-date")
const end_date = document.querySelector("#end-date")
const password = document.querySelector("#password")
const co2_box = document.querySelector("#co2-box")

const chart = new Chart(ctx, {
    type: 'line',
    options: {
        parsing: false,
        pointStyle: false,
        plugins: {
            tooltip: {
                intersect: false,
                mode: 'index',
            },
            zoom: {
                zoom: {
                    wheel: {
                        enabled: true,
                    },
                    pinch: {
                        enabled: true,
                    },
                    onZoomComplete: on_zoom_complete_check,
                },
                pan: {
                    enabled: true,
                    mode: 'xy',
                    onPanComplete: on_range_change,
                },
            }
        },
        scales: {
            xAxis: {
                type: "time",
                time: {
                    displayFormats: {
                        minute: "EEE HH:mm",
                        hour: "EEE HH:mm",
                        day: "EEE HH:mm",
                        week: "EEE HH:mm",
                    }
                }
            },
            y: {
                bounds: 'data',
                title: {
                    display: true,
                    text: 'Temperature (C)'
                },
                position: 'left',
            },
            y1: {
                bounds: 'data',
                display: true,
                position: 'right',
                title: {
                    display: true,
                    text: 'Humidity (%H)'
                },
            },
            y3: {
                display: true,
                min: 400,
                max: 2000,
                title: {
                    display: true,
                    text: 'Co2 (ppm)'
                }
            }
        }
    },
    data: {
        datasets: [],
    }
});

function report_error(message) {
    document.querySelectorAll('.hide-until-loaded').forEach((e) => {
        e.hidden = true;
    })

    error_message.textContent = message
    error_message.hidden = false
}

async function fetch_range_data(range) {
    const data = await fetch_data("/data/range/" + range)
    const temp = data.map((d) => { return { time: d.time, value: d.temperature } })
    const humid = data.map((d) => { return { time: d.time, value: d.humidity } })
    const co2 = data.filter((d) => d.co2).map((d) => { return { time: d.time, value: d.co2 } });

    return {
        temp: temp,
        humid: humid,
        co2: co2,
    }
}

async function fetch_range_data_between(start_ms, stop_ms) {
    const data = await fetch_data("/data/from/" + start_ms + "/to/" + stop_ms)
    const temp = data.map((d) => { return { time: d.time, value: d.temperature } })
    const humid = data.map((d) => { return { time: d.time, value: d.humidity } })
    const co2 = data.filter((d) => d.co2).map((d) => { return { time: d.time, value: d.co2 } });

    return {
        temp: temp,
        humid: humid,
        co2: co2,
    }
}

async function fetch_data(url) {
    const start = Date.now();

    let body

    try {
        const data = await fetch(url, { headers: { authorization: "Bearer " + password.value } })

        if (data.status != 200) {
            report_error(await data.text())
            return []
        }

        body = await data.json();

        const end = Date.now();

        load_time.textContent = end - start;
        points_loaded.textContent = body.length;

        error_message.hidden = true

        document.querySelectorAll('.hide-until-loaded').forEach((e) => {
            e.hidden = false;
        })

        if (body.length > 0) {
            start_date.textContent = new Date(body[0].time).toLocaleString()
            end_date.textContent = new Date(body[body.length - 1].time).toLocaleString()
        } else {
            start_date.hidden = true
            end_date.hidden = true
        }

        return body
    } catch (error) {
        report_error(error)
        return []
    } finally {
        document.querySelectorAll('.visible-until-loaded').forEach((e) => {
            e.hidden = true;
        })
    }
}

function update_chart(data) {
    const dataset = data.temp.map((d) => { return { x: d.time, y: d.value } })
    const dataset1 = data.humid.map((d) => { return { x: d.time, y: d.value } })
    const dataset2 = data.co2.map((d) => { return { x: d.time, y: d.value } })

    if (chart.data.datasets.length != 3) {
        chart.data.datasets.push({
            label: "Temperature (C)",
            data: dataset,
            yAxisID: 'y',
        })

        chart.data.datasets.push({
            label: "Humidity (%H)",
            data: dataset1,
            yAxisID: 'y1',
        })

        chart.data.datasets.push({
            label: "Co2 (ppm)",
            data: dataset2,
            yAxisID: 'y2',
        })
    } else {
        chart.data.datasets[0].data = dataset
        chart.data.datasets[1].data = dataset1
        chart.data.datasets[2].data = dataset2
    }

    chart.update('none')
}

timespan.onchange = () => {
    fetch_range_data(timespan.value).then(update_chart).then(() => chart.resetZoom())
}

password.onchange = () => {
    timespan.onchange()
}

reset.onclick = () => {
    timespan.onchange()
}

async function on_zoom_complete_check() {
    const axis = chart.scales.xAxis;
    const min = Math.round(axis.min)
    const max = Math.round(axis.max)

    update_chart(await fetch_range_data_between(min, max))
}

async function on_range_change() {
    const axis = chart.scales.xAxis;
    const min = Math.round(axis.min)
    const max = Math.round(axis.max)

    update_chart(await fetch_range_data_between(min, max))
}

function update_co2_box(co2_level) {
    console.log("current_co2 is fetched: ", co2_level);
    const current_level = co2_map.find((elem) => elem.start <= co2_level && co2_level < elem.end)
    console.log("current level: ", current_level);
    co2_box.textContent = current_level.description;
    co2_box.style.backgroundColor = current_level.color;
}

fetch("/co2/current").then((b) => b.json()).then(update_co2_box);
