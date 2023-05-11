window.addEventListener("DOMContentLoaded", () => {
  let params = (new URL(window.location)).searchParams;
  const buttons = document.getElementsByClassName('mood-button');
  for (let i = 0; i < buttons.length; i++) {
    let moodId = buttons[i].id.replace('-button', '');

    // set button state depending on get parameters
    let value = params.get(moodId);
    if (value == null) {
      if (moodId == 'weather') {
        ;
      }
      else {
        value = 'disable';
      }
    }
    buttons[i].classList.add("option-" + value);

    // set button click evnet
    //    🚶          🚶         🚶        ☂️        ☂️
    //    on         off       none       on        off
    // 🚶     ☂️   🚶    ☂️    🚶   ☂️   🚶   ☂️   🚶   ☂️ 
    // on   off   off  off   none *    none on    *   off
    buttons[i].addEventListener("click", () => {
      if (buttons[i].classList.contains('option-false')) {
        buttons[i].classList.remove('option-false');
        if (moodId == 'weather') {
          // set on
          document.getElementById('walking-button').classList.remove('option-false');
          document.getElementById('walking-button').classList.add('option-disable');
        }
        else {
          // set none
          buttons[i].classList.add('option-disable');
        }
      }
      else if (buttons[i].classList.contains('option-disable')) {
        // set on
        buttons[i].classList.remove('option-disable');
        if (moodId == 'walking') {
          document.getElementById('weather-button').classList.add('option-false');
        }
      }
      else {
        // set off
        buttons[i].classList.add('option-false');
        if (moodId == 'walking') {
          document.getElementById('weather-button').classList.add('option-false');
        }
      }
    });
  }

  // set parameter and reqest get method
  const refresh = document.getElementById('refresh');
  refresh.addEventListener("click", () => {
    let url = window.location.protocol + '//' + window.location.host + window.location.pathname;
    url = url + '?'
    for (let i = 0; i < buttons.length; i++) {
      let key = buttons[i].id.replace('-button', '');
      if (buttons[i].classList.contains('option-disable')) {
        ;
      } 
      else if (buttons[i].classList.contains('option-false')) {
        url += key + '=' + 'false&';
      } 
      else {
        url += key + '=' + 'true&';
      }
    }
    console.log(url);
    window.location = url;
  });
})