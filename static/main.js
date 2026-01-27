function showUploadForm() {
    document.getElementById('uploadForm').style.display = 'block';
}

function updateCounter() {
    const textarea = document.getElementById('postText');
    const counter = document.getElementById('counter');
    const currentLength = textarea.value.length;
    const maxLength = textarea.maxLength;
    counter.textContent = currentLength + '/' + maxLength;
    
    if (currentLength > maxLength * 0.9) {
        counter.style.color = 'red';
    } else {
        counter.style.color = 'black';
    }
}

async function uploadFile() {
    const fileInput = document.getElementById('fileInput');
    const formData = new FormData();

    for (let file of fileInput.files) {
        formData.append('files', file);
    }
    
    const textInput = document.getElementById('postText');
    formData.append('text', textInput.value);
    
    const response = await fetch('/upload', {
        method: 'POST',
        body: formData
    });

    if (response.ok) {
        alert('Файлы загружены!');
    }
}

document.addEventListener('DOMContentLoaded', function() {
    updateCounter();
});