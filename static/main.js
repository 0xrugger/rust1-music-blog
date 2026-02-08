function updateCounter() {
    const textarea = document.getElementById('postInput');
    const counter = document.getElementById('counter');
    
    if (!textarea || !counter) return;
    
    const currentLength = textarea.value.length;
    const maxLength = textarea.maxLength;
    counter.textContent = currentLength + '/' + maxLength;
    
    if (currentLength > maxLength * 0.9) {
        counter.style.color = 'red';
    } else {
        counter.style.color = '#dadada';
    }
}
function validateForm() {
    const titleInput = document.querySelector('input[name="title"]');
    const textInput = document.getElementById('postInput');
    
    if (!titleInput || !textInput) return true;
    
    if (titleInput.value.trim() === '') {
        alert('Please enter a title');
        titleInput.focus();
        return false;
    }
    
    if (textInput.value.trim() === '') {
        alert('Please enter post text');
        textInput.focus();
        return false;
    }
    
    if (textInput.value.length > textInput.maxLength) {
        alert('Post text is too long. Maximum length is ' + textInput.maxLength + ' characters.');
        textInput.focus();
        return false;
    }
    
    return true;
}
function updatePostCount(count) {
    const postCountElement = document.getElementById('postCount');
    if (postCountElement) {
        postCountElement.textContent = count;
    }
}


function animateButton(button) {
    button.style.transform = 'scale(0.95)';
    setTimeout(() => {
        button.style.transform = 'scale(1)';
    }, 150);
}

function closeModalAndCleanUrl() {
    const modal = document.getElementById('uploadSuccessModal');
    if (modal) {
        modal.style.display = 'none';
    }
    
    if (window.history.replaceState) {
        const newUrl = window.location.pathname;
        window.history.replaceState({}, document.title, newUrl);
    }
}

document.addEventListener('DOMContentLoaded', function() {
    const textarea = document.getElementById('postInput');
    if (textarea) {
        updateCounter(); 
        textarea.addEventListener('input', updateCounter); 
    }

    const submitButton = document.getElementById('submitPost');
    if (submitButton) {
        submitButton.addEventListener('click', function(event) {
            if (validateForm()) {
                animateButton(this);
            }
        });
    }

    
    const postItems = document.querySelectorAll('.post-item');
    postItems.forEach(post => {
        post.addEventListener('mouseenter', function() {
            this.style.transition = 'all 0.3s ease';
        });
    });

    const modalCloseBtn = document.querySelector('.modal-close-btn');
    if (modalCloseBtn) {
        modalCloseBtn.addEventListener('click', closeModalAndCleanUrl);
    }
    
    const postCount = document.querySelector('#postCount')?.textContent || '0';
    updatePostCount(postCount);
    
    const formInputs = document.querySelectorAll('.form-input, .form-textarea');
    formInputs.forEach(input => {
        input.addEventListener('focus', function() {
            this.parentElement.style.borderColor = 'var(--primary-color)';
        });
        
        input.addEventListener('blur', function() {
            this.parentElement.style.borderColor = '';
        });
    });
});