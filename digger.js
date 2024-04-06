var users;

document.addEventListener('DOMContentLoaded', () => {

  // Get all "navbar-burger" elements
  const $navbarBurgers = Array.prototype.slice.call(document.querySelectorAll('.navbar-burger'), 0);

  // Add a click event on each of them
  $navbarBurgers.forEach( el => {
    el.addEventListener('click', () => {

      // Get the target from the "data-target" attribute
      const target = el.dataset.target;
      const $target = document.getElementById(target);

      // Toggle the "is-active" class on both the "navbar-burger" and the "navbar-menu"
      el.classList.toggle('is-active');
      $target.classList.toggle('is-active');

    });
  });
});

function search_user() {
  let text = document.getElementById("user-search").value;
  //console.log("search_user", text);
  text = text.toLowerCase();
  //if (text.length < 3) {
  //    console.log("short");
  //    return;
  //}
  let selected_users = users.filter(function (entry) {
      return entry["name"].toLowerCase().includes(text) || entry["gh_login"].toLowerCase().includes(text);
  });
  //console.log(selected_users.length);
  let limit = Math.min(selected_users.length, 10);
  let html = selected_users.slice(0, limit).map(function (entry) {
       return `<tr><td>${entry["name"]}</td><td><a href="/users/${ entry["gh_login"].toLowerCase() }">${entry["gh_login"]}</a></td></tr>`;
  }).join("");
  //console.log(html);
  document.getElementById("total").innerHTML = `Total: ${users.length} Selected: ${selected_users.length} Showing: ${limit}`;
  document.getElementById("mytable").innerHTML = html;
}

function fetchJSONFile(path, callback) {
    var httpRequest = new XMLHttpRequest();
    httpRequest.onreadystatechange = function() {
        if (httpRequest.readyState === 4) {
            if (httpRequest.status === 200) {
                //console.log(httpRequest.responseText);
                var data = JSON.parse(httpRequest.responseText);
                if (callback) callback(data);
            }
        }
    };
    httpRequest.open('GET', path);
    httpRequest.send();
}


document.addEventListener('DOMContentLoaded', () => {
    let user_search_box = document.getElementById("user-search");
    if (user_search_box) {
        fetchJSONFile('/users.json', function(data){
            users = data;
            console.log("loaded");
            //console.log(user);
        });

        user_search_box.addEventListener('input', search_user);
    }
});

